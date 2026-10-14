//! The open gauge definition: elements, editor-only state (groups, locks,
//! hidden elements), named undo history and the selection-independent edits.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gauge_render::model::*;
use serde::{Deserialize, Serialize};

/// Editor-only state, kept next to the definition in `<name>.designer.json`
/// so gauge files stay exactly what the device reads.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct Meta {
	/// Elements that can't be picked or moved on the canvas.
	pub locked: BTreeSet<String>,
	/// Elements left out of the preview (they still render on the device).
	pub hidden: BTreeSet<String>,
	/// Group number per element `Id`.
	pub groups: BTreeMap<String, u32>,
}

impl Meta {
	pub fn is_empty(&self) -> bool {
		self.locked.is_empty() && self.hidden.is_empty() && self.groups.is_empty()
	}

	/// Drops entries for elements that no longer exist.
	fn retain_ids(&mut self, ids: &BTreeSet<&str>) {
		self.locked.retain(|id| ids.contains(id.as_str()));
		self.hidden.retain(|id| ids.contains(id.as_str()));
		self.groups.retain(|id, _| ids.contains(id.as_str()));
		// A group of one is no group.
		let mut counts: BTreeMap<u32, usize> = BTreeMap::new();
		for g in self.groups.values() {
			*counts.entry(*g).or_default() += 1;
		}
		self.groups.retain(|_, g| counts[g] > 1);
	}

	pub fn sidecar_path(json: &Path) -> PathBuf {
		let stem = json.file_stem().and_then(|s| s.to_str()).unwrap_or("gauge");
		json.with_file_name(format!("{stem}.designer.json"))
	}
}

#[derive(Clone)]
struct State {
	def: CustomGaugeDefinition,
	meta: Meta,
}

struct Entry {
	name: String,
	state: State,
}

/// Snapshot history with action names. Edits of the same property in a
/// row (dragging a slider, typing) merge into one step.
#[derive(Default)]
pub struct History {
	undo: Vec<Entry>,
	redo: Vec<Entry>,
	/// Key and time of the last merged edit.
	merge: Option<(String, Instant)>,
}

const MAX_HISTORY: usize = 200;
const MERGE_WINDOW: Duration = Duration::from_millis(1500);

pub struct Document {
	pub def: CustomGaugeDefinition,
	pub meta: Meta,
	pub path: Option<PathBuf>,
	history: History,
	/// Bumped on every change; the preview re-renders when it moves.
	pub revision: u64,
	saved_revision: u64,
	/// Used for default element names (`Arc 3`).
	counter: usize,
}

impl Document {
	pub fn new(def: CustomGaugeDefinition) -> Self {
		let counter = def.elements.len();
		Self {
			def,
			meta: Meta::default(),
			path: None,
			history: History::default(),
			revision: 1,
			saved_revision: 1,
			counter,
		}
	}

	/// The C# designer's starting gauge: an RPM dial.
	pub fn default_gauge() -> CustomGaugeDefinition {
		crate::templates::rpm_dial()
	}

	pub fn open(path: &Path) -> Result<Self, String> {
		let def = CustomGaugeDefinition::load(path)?;
		let mut doc = Self::new(def);
		doc.path = Some(path.to_path_buf());
		if let Ok(json) = std::fs::read_to_string(Meta::sidecar_path(path)) {
			doc.meta = serde_json::from_str(&json).unwrap_or_default();
			doc.clean_meta();
		}
		Ok(doc)
	}

	pub fn is_dirty(&self) -> bool {
		self.revision != self.saved_revision
	}

	pub fn mark_saved(&mut self) {
		self.saved_revision = self.revision;
	}

	pub fn touch(&mut self) {
		self.revision += 1;
	}

	pub fn title(&self) -> String {
		let name = self.path.as_deref().and_then(Path::file_name).and_then(|n| n.to_str()).unwrap_or("Untitled");
		format!("{name}{}", if self.is_dirty() { " •" } else { "" })
	}

	// ── Undo ──

	fn state(&self) -> State {
		State { def: self.def.clone(), meta: self.meta.clone() }
	}

	fn restore(&mut self, state: State) {
		let base_dir = self.def.base_directory.clone();
		self.def = state.def;
		self.def.base_directory = base_dir;
		self.meta = state.meta;
		self.touch();
	}

	/// Records the state before an edit named `name` ("Move Arc").
	pub fn checkpoint(&mut self, name: impl Into<String>) {
		self.history.merge = None;
		self.push_undo(name.into());
	}

	/// Like [`checkpoint`](Self::checkpoint), but consecutive edits with the
	/// same `key` within a moment merge into one undo step.
	pub fn checkpoint_merged(&mut self, name: impl Into<String>, key: &str) {
		let now = Instant::now();
		if let Some((last, at)) = &self.history.merge {
			if last == key && now.duration_since(*at) < MERGE_WINDOW && !self.history.undo.is_empty() {
				self.history.merge = Some((key.to_string(), now));
				self.touch();
				return;
			}
		}
		self.push_undo(name.into());
		self.history.merge = Some((key.to_string(), now));
	}

	fn push_undo(&mut self, name: String) {
		let state = self.state();
		self.history.undo.push(Entry { name, state });
		if self.history.undo.len() > MAX_HISTORY {
			self.history.undo.remove(0);
		}
		self.history.redo.clear();
		self.touch();
	}

	pub fn undo_name(&self) -> Option<&str> {
		self.history.undo.last().map(|e| e.name.as_str())
	}

	pub fn redo_name(&self) -> Option<&str> {
		self.history.redo.last().map(|e| e.name.as_str())
	}

	/// Undoes the last step; returns its name.
	pub fn undo(&mut self) -> Option<String> {
		let entry = self.history.undo.pop()?;
		self.history.redo.push(Entry { name: entry.name.clone(), state: self.state() });
		self.history.merge = None;
		self.restore(entry.state);
		Some(entry.name)
	}

	pub fn redo(&mut self) -> Option<String> {
		let entry = self.history.redo.pop()?;
		self.history.undo.push(Entry { name: entry.name.clone(), state: self.state() });
		self.history.merge = None;
		self.restore(entry.state);
		Some(entry.name)
	}

	/// Names of the undo steps, oldest first.
	pub fn history_names(&self) -> impl Iterator<Item = &str> {
		self.history.undo.iter().map(|e| e.name.as_str())
	}

	// ── Elements ──

	pub fn index_of(&self, id: &str) -> Option<usize> {
		self.def.elements.iter().position(|e| e.base().id == id)
	}

	pub fn element(&self, id: &str) -> Option<&Element> {
		self.def.elements.iter().find(|e| e.base().id == id)
	}

	pub fn element_mut(&mut self, id: &str) -> Option<&mut Element> {
		self.def.elements.iter_mut().find(|e| e.base().id == id)
	}

	/// "Arc 'RPM Arc'" style label for messages.
	pub fn describe(&self, id: &str) -> String {
		self.element(id).map(describe).unwrap_or_else(|| "element".into())
	}

	/// Adds a new element of type `tag` (`"arc"`, ...) and returns its id.
	pub fn add(&mut self, tag: &str) -> Option<String> {
		let mut element: Element = serde_json::from_value(serde_json::json!({ "$type": tag })).ok()?;
		self.counter += 1;
		element.base_mut().name = format!("{} {}", element.type_label(), self.counter);
		self.checkpoint(format!("Add {}", element.type_label()));
		let id = element.base().id.clone();
		self.def.elements.push(element);
		Some(id)
	}

	/// Inserts copies of `elements` with fresh ids (paste, duplicate,
	/// snippets), keeping groups among them. Returns the new ids.
	pub fn insert_copies(
		&mut self,
		elements: &[Element],
		groups: &BTreeMap<String, u32>,
		offset: f32,
		name: &str,
	) -> Vec<String> {
		if elements.is_empty() {
			return Vec::new();
		}
		self.checkpoint(name);
		let mut group_map: BTreeMap<u32, u32> = BTreeMap::new();
		let mut next_group = self.meta.groups.values().max().copied().unwrap_or(0) + 1;
		let mut ids = Vec::new();
		for element in elements {
			let mut copy = element.clone();
			let old_id = copy.base().id.clone();
			let id = fresh_id();
			copy.base_mut().id = id.clone();
			crate::geometry::translate(&mut copy, offset, offset);
			if let Some(g) = groups.get(&old_id) {
				let new_group = *group_map.entry(*g).or_insert_with(|| {
					next_group += 1;
					next_group - 1
				});
				self.meta.groups.insert(id.clone(), new_group);
			}
			self.def.elements.push(copy);
			ids.push(id);
		}
		self.clean_meta();
		ids
	}

	pub fn delete(&mut self, ids: &[String]) {
		if ids.is_empty() {
			return;
		}
		let name = if ids.len() == 1 {
			format!("Delete {}", self.describe(&ids[0]))
		} else {
			format!("Delete {} elements", ids.len())
		};
		self.checkpoint(name);
		self.def.elements.retain(|e| !ids.contains(&e.base().id));
		self.clean_meta();
	}

	/// Moves the selected elements one step through the draw order
	/// (`-1` = towards the back, as the C# "Move Up").
	pub fn move_in_order(&mut self, ids: &[String], direction: i32) {
		let mut idx: Vec<usize> = ids.iter().filter_map(|id| self.index_of(id)).collect();
		if idx.is_empty() {
			return;
		}
		idx.sort_unstable();
		let n = self.def.elements.len();
		let blocked = if direction < 0 { idx[0] == 0 } else { *idx.last().unwrap() + 1 >= n };
		if blocked {
			return;
		}
		self.checkpoint(if direction < 0 { "Move back" } else { "Move forward" });
		if direction < 0 {
			for i in idx {
				self.def.elements.swap(i, i - 1);
			}
		} else {
			for i in idx.into_iter().rev() {
				self.def.elements.swap(i, i + 1);
			}
		}
	}

	/// Moves the selected elements to the front or back of the draw order.
	pub fn move_to_end(&mut self, ids: &[String], front: bool) {
		if ids.is_empty() {
			return;
		}
		self.checkpoint(if front { "Bring to front" } else { "Send to back" });
		let (mut picked, rest): (Vec<Element>, Vec<Element>) =
			std::mem::take(&mut self.def.elements).into_iter().partition(|e| ids.contains(&e.base().id));
		self.def.elements = if front {
			let mut v = rest;
			v.append(&mut picked);
			v
		} else {
			picked.extend(rest);
			picked
		};
	}

	// ── Groups, locks, visibility ──

	pub fn group_of(&self, id: &str) -> Option<u32> {
		self.meta.groups.get(id).copied()
	}

	/// Ids of every element in the same group (or just `id`).
	pub fn group_members(&self, id: &str) -> Vec<String> {
		match self.group_of(id) {
			Some(g) => self
				.def
				.elements
				.iter()
				.map(|e| &e.base().id)
				.filter(|i| self.meta.groups.get(*i) == Some(&g))
				.cloned()
				.collect(),
			None => vec![id.to_string()],
		}
	}

	pub fn group(&mut self, ids: &[String]) {
		if ids.len() < 2 {
			return;
		}
		self.checkpoint("Group");
		let g = self.meta.groups.values().max().copied().unwrap_or(0) + 1;
		for id in ids {
			self.meta.groups.insert(id.clone(), g);
		}
	}

	pub fn ungroup(&mut self, ids: &[String]) {
		if !ids.iter().any(|id| self.meta.groups.contains_key(id)) {
			return;
		}
		self.checkpoint("Ungroup");
		for id in ids {
			self.meta.groups.remove(id);
		}
		self.clean_meta();
	}

	pub fn set_locked(&mut self, ids: &[String], locked: bool) {
		self.checkpoint(if locked { "Lock" } else { "Unlock" });
		for id in ids {
			if locked {
				self.meta.locked.insert(id.clone());
			} else {
				self.meta.locked.remove(id);
			}
		}
	}

	pub fn set_hidden(&mut self, ids: &[String], hidden: bool) {
		self.checkpoint(if hidden { "Hide" } else { "Show" });
		for id in ids {
			if hidden {
				self.meta.hidden.insert(id.clone());
			} else {
				self.meta.hidden.remove(id);
			}
		}
	}

	pub fn is_locked(&self, id: &str) -> bool {
		self.meta.locked.contains(id)
	}

	pub fn is_hidden(&self, id: &str) -> bool {
		self.meta.hidden.contains(id)
	}

	fn clean_meta(&mut self) {
		let ids: BTreeSet<&str> = self.def.elements.iter().map(|e| e.base().id.as_str()).collect();
		self.meta.retain_ids(&ids);
	}

	/// The definition the preview draws: hidden elements left out.
	pub fn preview_definition(&self) -> CustomGaugeDefinition {
		let mut def = self.def.clone();
		if !self.meta.hidden.is_empty() {
			def.elements.retain(|e| !self.meta.hidden.contains(&e.base().id));
		}
		def
	}

	// ── Data sources ──

	/// Channel names test sliders are needed for, with the widest min/max of
	/// the elements bound to each: element sources, visibility sources and
	/// calculated-channel inputs. Calculated channels themselves are computed.
	pub fn bound_sources(&self) -> Vec<(String, f32, f32)> {
		let is_channel = |name: &str| self.def.calculated_channels.iter().any(|c| c.name.eq_ignore_ascii_case(name));
		let mut out: Vec<(String, f32, f32)> = Vec::new();
		let mut add = |name: &str, min: f32, max: f32| {
			if name.is_empty() || is_channel(name) {
				return;
			}
			if let Some(entry) = out.iter_mut().find(|(n, _, _)| n.eq_ignore_ascii_case(name)) {
				entry.1 = entry.1.min(min);
				entry.2 = entry.2.max(max);
			} else {
				out.push((name.to_string(), min, max));
			}
		};
		for element in &self.def.elements {
			let b = element.base();
			if let Some(s) = b.source() {
				add(s, b.min_value, b.max_value);
			}
		}
		for element in &self.def.elements {
			let b = element.base();
			if b.use_visibility {
				if let Some(s) = b.visibility_source.as_deref() {
					let lo = if b.visible_above > f32::MIN { b.visible_above - 10.0 } else { 0.0 };
					let hi = if b.visible_below < f32::MAX { b.visible_below + 10.0 } else { 100.0 };
					add(s, lo.min(hi), hi.max(lo));
				}
			}
		}
		for ch in &self.def.calculated_channels {
			for name in gauge_render::expression::identifiers(&ch.expression) {
				add(&name, 0.0, 100.0);
			}
		}
		out
	}
}

/// "Arc 'RPM Arc'".
pub fn describe(element: &Element) -> String {
	let name = &element.base().name;
	if name.is_empty() {
		element.type_label().to_string()
	} else {
		format!("{} '{}'", element.type_label(), name)
	}
}

/// An 8-hex-digit id like the C# `Guid.NewGuid().ToString("N")[..8]`.
pub fn fresh_id() -> String {
	// The model's default constructor already makes unique ids.
	ElementBase::default().id
}

// ── Clipboard ──

/// What copy puts on the clipboard: a JSON object other designer windows
/// (and files) can paste.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Clip {
	pub elements: Vec<Element>,
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub groups: BTreeMap<String, u32>,
}

impl Clip {
	pub fn from_selection(doc: &Document, ids: &[String]) -> Self {
		// Keep draw order, not click order.
		let elements: Vec<Element> = doc.def.elements.iter().filter(|e| ids.contains(&e.base().id)).cloned().collect();
		let groups = doc.meta.groups.iter().filter(|(id, _)| ids.contains(id)).map(|(k, v)| (k.clone(), *v)).collect();
		Self { elements, groups }
	}

	pub fn to_json(&self) -> String {
		serde_json::to_string_pretty(self).unwrap_or_default()
	}

	/// Parses pasted text: a clip, a whole gauge definition, an element
	/// list or a single element.
	pub fn parse(text: &str) -> Option<Self> {
		let text = text.trim().trim_start_matches('\u{feff}');
		if let Ok(clip) = serde_json::from_str::<Clip>(text) {
			return Some(clip);
		}
		if let Ok(elements) = serde_json::from_str::<Vec<Element>>(text) {
			return Some(Self { elements, groups: BTreeMap::new() });
		}
		if let Ok(element) = serde_json::from_str::<Element>(text) {
			return Some(Self { elements: vec![element], groups: BTreeMap::new() });
		}
		serde_json::from_str::<CustomGaugeDefinition>(text)
			.ok()
			.filter(|d| !d.elements.is_empty())
			.map(|d| Self { elements: d.elements, groups: BTreeMap::new() })
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn doc_with(n: usize) -> (Document, Vec<String>) {
		let mut doc = Document::new(CustomGaugeDefinition::default());
		let ids = (0..n).map(|_| doc.add("circle").unwrap()).collect();
		(doc, ids)
	}

	#[test]
	fn undo_and_redo_have_names() {
		let (mut doc, ids) = doc_with(1);
		doc.checkpoint("Move Circle");
		crate::geometry::translate(doc.element_mut(&ids[0]).unwrap(), 10.0, 0.0);
		assert_eq!(doc.undo_name(), Some("Move Circle"));
		assert_eq!(doc.undo().as_deref(), Some("Move Circle"));
		assert_eq!(doc.element(&ids[0]).unwrap().base().x, 320.0);
		assert_eq!(doc.redo().as_deref(), Some("Move Circle"));
		assert_eq!(doc.element(&ids[0]).unwrap().base().x, 330.0);
		assert_eq!(doc.undo().as_deref(), Some("Move Circle"));
		assert_eq!(doc.undo().as_deref(), Some("Add Circle"));
		assert!(doc.def.elements.is_empty());
	}

	#[test]
	fn repeated_edits_of_one_property_merge() {
		let (mut doc, _) = doc_with(0);
		doc.checkpoint_merged("Change Radius", "x/Radius");
		doc.checkpoint_merged("Change Radius", "x/Radius");
		doc.checkpoint_merged("Change Colour", "x/Color");
		assert_eq!(doc.history_names().collect::<Vec<_>>(), ["Change Radius", "Change Colour"]);
	}

	#[test]
	fn copies_get_new_ids_and_keep_their_groups() {
		let (mut doc, ids) = doc_with(3);
		doc.group(&ids[..2]);
		let clip = Clip::parse(&Clip::from_selection(&doc, &ids[..2]).to_json()).unwrap();
		let new_ids = doc.insert_copies(&clip.elements, &clip.groups, 20.0, "Paste");
		assert_eq!(new_ids.len(), 2);
		assert!(new_ids.iter().all(|id| !ids.contains(id)));
		assert_eq!(doc.group_members(&new_ids[0]), new_ids);
		assert_ne!(doc.group_of(&new_ids[0]), doc.group_of(&ids[0]));
		assert_eq!(doc.element(&new_ids[0]).unwrap().base().x, 340.0);
	}

	#[test]
	fn pastes_plain_element_json() {
		let clip = Clip::parse(r#"{ "$type": "text", "Text": "RPM" }"#).unwrap();
		assert_eq!(clip.elements.len(), 1);
		let clip = Clip::parse(r#"{ "Elements": [{ "$type": "arc" }, { "$type": "needle" }] }"#).unwrap();
		assert_eq!(clip.elements.len(), 2);
		assert!(Clip::parse("hello").is_none());
	}

	#[test]
	fn deleting_cleans_groups_and_locks() {
		let (mut doc, ids) = doc_with(3);
		doc.group(&ids[..2]);
		doc.set_locked(&ids[..1], true);
		doc.delete(&ids[..1]);
		assert!(doc.meta.is_empty(), "the group of one left is dissolved: {:?}", doc.meta);
	}

	#[test]
	fn draw_order() {
		let (mut doc, ids) = doc_with(3);
		doc.move_in_order(&ids[2..], -1);
		assert_eq!(doc.index_of(&ids[2]), Some(1));
		doc.move_to_end(&ids[..1], true);
		assert_eq!(doc.index_of(&ids[0]), Some(2));
	}

	#[test]
	fn sliders_cover_every_input() {
		let (mut doc, _) = doc_with(0);
		let mut arc = ArcElement::default();
		arc.base.data_source = Some("Rpm".into());
		arc.base.max_value = 8000.0;
		doc.def.elements.push(Element::Arc(arc));
		doc.def.calculated_channels.push(CalculatedChannel { name: "Boost".into(), expression: "Map - Baro".into() });
		let mut bar = LinearBarElement::default();
		bar.base.data_source = Some("Boost".into());
		doc.def.elements.push(Element::LinearBar(bar));
		let names: Vec<_> = doc.bound_sources().into_iter().map(|s| s.0).collect();
		assert_eq!(names, ["Rpm", "Map", "Baro"]);
	}
}
