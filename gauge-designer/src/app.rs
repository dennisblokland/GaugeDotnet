//! The designer window.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{
	self, pos2, vec2, Align2, Color32, CursorIcon, FontId, Key, Modifiers, PointerButton, Pos2, Rect, RichText, Sense,
	Shape, Stroke, TextureHandle, TextureOptions, Ui, Vec2,
};
use gauge_render::model::*;
use gauge_render::{Canvas, ElementRenderer, Values};

use crate::config_editor::ConfigEditor;
use crate::cost::{self, CostReport};
use crate::deploy::{self, DeployTarget};
use crate::document::{describe, Clip, Document};
use crate::fields::{self, TYPES};
use crate::geometry::{self, AngleHandle, Bounds};
use crate::preview::{self, PreviewOptions};
use crate::props::{self, PropContext};
use crate::validate::{self, Issue, Severity};

const RULER: f32 = 18.0;
/// Pointer distance (screen px) that counts as on a handle.
const HANDLE_HIT: f32 = 8.0;
/// Smart guides snap within this many screen px.
const GUIDE_SNAP: f32 = 6.0;
const SELECT_COLOR: Color32 = Color32::from_rgb(0x4D, 0xC3, 0xFF);
const SECONDARY_COLOR: Color32 = Color32::from_rgb(0xFF, 0xC8, 0x00);
const GUIDE_COLOR: Color32 = Color32::from_rgb(0xFF, 0x3D, 0xA0);

enum Interaction {
	None,
	/// Moving the selection; `origins` are the elements as they were.
	Move {
		start: Pos2,
		origins: Vec<(String, Element)>,
		moved: bool,
	},
	Resize {
		id: String,
		corner: usize,
		start: Bounds,
		origin: Box<Element>,
		moved: bool,
	},
	Angle {
		id: String,
		handle: AngleHandle,
		moved: bool,
	},
	Marquee {
		start: Pos2,
		current: Pos2,
		additive: bool,
	},
	Pan,
}

/// Something to do once unsaved changes are dealt with.
#[derive(Clone)]
enum Pending {
	New(usize),
	Open(Option<PathBuf>),
	Quit,
}

/// Result of an upload: (log, `CustomDefinitionFile` value) or (log, error).
type UploadResult = Result<(String, String), (String, String)>;

struct Job<T> {
	label: String,
	handle: std::thread::JoinHandle<T>,
}

pub struct DesignerApp {
	doc: Document,
	/// Selected element ids; the last one is the primary (its properties show).
	selection: Vec<String>,
	renderer: ElementRenderer,
	canvas: Canvas,
	texture: Option<TextureHandle>,
	texture_filter: Option<TextureOptions>,
	rendered: Option<(u64, u64, PreviewOptions)>,
	last_clock_second: i64,
	values: Values,
	values_revision: u64,
	animate: bool,
	anim_start: Instant,

	zoom: f32,
	pan: Vec2,
	fit_pending: bool,
	grid_size: f32,
	snap_to_grid: bool,
	show_grid: bool,
	smart_guides: bool,
	rulers: bool,
	interaction: Interaction,
	guides: Vec<(bool, f32)>,
	cursor: Option<Pos2>,

	clipboard: Option<String>,
	message: Option<(String, Instant, bool)>,
	issues: Vec<Issue>,
	issues_revision: u64,
	cost: Option<CostReport>,
	cost_revision: u64,
	auto_cost: bool,
	preview: PreviewOptions,
	show_history: bool,

	pending: Option<Pending>,
	allow_close: bool,
	title: String,
	last_dir: Option<PathBuf>,

	config: Option<ConfigEditor>,
	deploy_open: bool,
	deploy_target: DeployTarget,
	deploy_subdir: String,
	deploy_log: String,
	deploy_job: Option<Job<UploadResult>>,
	export_job: Option<Job<Result<PathBuf, String>>>,
}

impl DesignerApp {
	pub fn new(path: Option<PathBuf>, config: Option<PathBuf>) -> Self {
		let mut app = Self {
			doc: Document::new(Document::default_gauge()),
			selection: Vec::new(),
			renderer: ElementRenderer::new(),
			canvas: Canvas::new(640, 480),
			texture: None,
			texture_filter: None,
			rendered: None,
			last_clock_second: 0,
			values: Values::new(),
			values_revision: 0,
			animate: false,
			anim_start: Instant::now(),
			zoom: 1.0,
			pan: Vec2::ZERO,
			fit_pending: true,
			grid_size: 10.0,
			snap_to_grid: false,
			show_grid: false,
			smart_guides: true,
			rulers: true,
			interaction: Interaction::None,
			guides: Vec::new(),
			cursor: None,
			clipboard: None,
			message: None,
			issues: Vec::new(),
			issues_revision: 0,
			cost: None,
			cost_revision: 0,
			auto_cost: false,
			preview: PreviewOptions::default(),
			show_history: false,
			pending: None,
			allow_close: false,
			title: String::new(),
			last_dir: None,
			config: None,
			deploy_open: false,
			deploy_target: DeployTarget::from_env(),
			deploy_subdir: String::new(),
			deploy_log: String::new(),
			deploy_job: None,
			export_job: None,
		};
		app.doc.mark_saved();
		if let Some(path) = path {
			app.open_path(&path);
		}
		if let Some(path) = config {
			app.config = Some(ConfigEditor::open(&path));
		}
		app
	}

	fn notify(&mut self, text: impl Into<String>) {
		self.message = Some((text.into(), Instant::now(), false));
	}

	fn error(&mut self, text: impl Into<String>) {
		self.message = Some((text.into(), Instant::now(), true));
	}

	fn primary(&self) -> Option<&str> {
		self.selection.last().map(String::as_str)
	}

	fn select_only(&mut self, ids: Vec<String>) {
		self.selection = ids;
	}

	fn prune_selection(&mut self) {
		let doc = &self.doc;
		self.selection.retain(|id| doc.index_of(id).is_some());
	}

	fn rerender_values(&mut self) {
		self.values_revision += 1;
	}

	// ── Files ──

	fn guard(&mut self, action: Pending) {
		if self.doc.is_dirty() {
			self.pending = Some(action);
		} else {
			self.run_pending(action);
		}
	}

	fn run_pending(&mut self, action: Pending) {
		match action {
			Pending::New(template) => {
				let def = (crate::templates::TEMPLATES[template].1)();
				self.doc = Document::new(def);
				self.after_load();
			}
			Pending::Open(path) => {
				let path = path.or_else(|| self.dialog().add_filter("Gauge JSON", &["json"]).pick_file());
				if let Some(path) = path {
					self.open_path(&path);
				}
			}
			Pending::Quit => {
				self.allow_close = true;
			}
		}
	}

	fn dialog(&self) -> rfd::FileDialog {
		let dialog = rfd::FileDialog::new();
		match self.last_dir.as_deref().or(self.doc.def.base_directory.as_deref()) {
			Some(dir) => dialog.set_directory(dir),
			None => dialog,
		}
	}

	fn open_path(&mut self, path: &Path) {
		match Document::open(path) {
			Ok(doc) => {
				self.doc = doc;
				self.last_dir = path.parent().map(Path::to_path_buf);
				self.after_load();
				self.notify(format!("Opened {}", path.display()));
			}
			Err(e) => self.error(format!("Couldn't open: {e}")),
		}
	}

	fn after_load(&mut self) {
		self.selection.clear();
		self.renderer.clear_caches();
		self.rendered = None;
		self.cost = None;
		self.fit_pending = true;
		self.values.clear();
		self.rerender_values();
	}

	fn save(&mut self, save_as: bool) -> bool {
		let path = match (&self.doc.path, save_as) {
			(Some(p), false) => p.clone(),
			_ => match self.dialog().add_filter("Gauge JSON", &["json"]).set_file_name("custom-gauge.json").save_file()
			{
				Some(p) => p,
				None => return false,
			},
		};
		match crate::package::save(&self.doc.def, &path) {
			Ok(saved) => {
				self.doc.def = saved.definition;
				self.doc.path = Some(path.clone());
				let sidecar = crate::document::Meta::sidecar_path(&path);
				if self.doc.meta.is_empty() {
					let _ = std::fs::remove_file(&sidecar);
				} else if let Err(e) = crate::package::write_atomic(
					&sidecar,
					serde_json::to_string_pretty(&self.doc.meta).unwrap_or_default().as_bytes(),
				) {
					self.error(format!("Saved, but the editor state wasn't: {e}"));
				}
				self.doc.touch();
				self.doc.mark_saved();
				self.renderer.clear_caches();
				self.last_dir = path.parent().map(Path::to_path_buf);
				if saved.warnings.is_empty() {
					self.notify(format!("Saved {}", path.display()));
				} else {
					self.error(format!("Saved with problems: {}", saved.warnings.join("; ")));
				}
				true
			}
			Err(e) => {
				self.error(format!("Save failed: {e}"));
				false
			}
		}
	}

	fn insert_snippet(&mut self) {
		let Some(path) = self.dialog().add_filter("Gauge or snippet JSON", &["json"]).pick_file() else { return };
		let result = std::fs::read_to_string(&path)
			.map_err(|e| e.to_string())
			.and_then(|text| Clip::parse(&text).ok_or_else(|| "no elements in this file".to_string()));
		match result {
			Ok(mut clip) => {
				// Relative image paths in the snippet resolve against its folder.
				if let Some(dir) = path.parent() {
					absolutize_images(&mut clip.elements, dir);
				}
				let ids = self.doc.insert_copies(&clip.elements, &clip.groups, 0.0, "Insert snippet");
				self.notify(format!("Inserted {} elements", ids.len()));
				self.select_only(ids);
			}
			Err(e) => self.error(format!("Couldn't insert {}: {e}", path.display())),
		}
	}

	fn save_snippet(&mut self) {
		if self.selection.is_empty() {
			return;
		}
		let Some(path) = self.dialog().add_filter("Snippet JSON", &["json"]).set_file_name("snippet.json").save_file()
		else {
			return;
		};
		let mut clip = Clip::from_selection(&self.doc, &self.selection);
		if let Some(dir) = self.doc.def.base_directory.clone() {
			absolutize_images(&mut clip.elements, &dir);
		}
		match crate::package::write_atomic(&path, clip.to_json().as_bytes()) {
			Ok(()) => self.notify(format!("Saved {} elements to {}", clip.elements.len(), path.display())),
			Err(e) => self.error(e),
		}
	}

	fn export_png(&mut self) {
		let Some(path) = self.dialog().add_filter("PNG", &["png"]).set_file_name("gauge.png").save_file() else {
			return;
		};
		match preview::export_png(&self.canvas, &path) {
			Ok(()) => self.notify(format!("Exported {}", path.display())),
			Err(e) => self.error(e),
		}
	}

	fn export_gif(&mut self) {
		let Some(path) = self.dialog().add_filter("GIF", &["gif"]).set_file_name("gauge.gif").save_file() else {
			return;
		};
		let def = self.doc.preview_definition();
		let sources = self.doc.bound_sources();
		let options = self.preview;
		let handle = std::thread::spawn(move || preview::export_gif(&def, &sources, &options, 60, &path).map(|_| path));
		self.export_job = Some(Job { label: "Exporting GIF…".into(), handle });
	}

	// ── Edit actions ──

	fn add_element(&mut self, tag: &str) {
		if let Some(id) = self.doc.add(tag) {
			self.select_only(vec![id]);
		}
	}

	fn copy(&mut self, ctx: &egui::Context) {
		if self.selection.is_empty() {
			return;
		}
		let mut clip = Clip::from_selection(&self.doc, &self.selection);
		// Absolute image paths, so a paste into another file still finds them.
		if let Some(dir) = self.doc.def.base_directory.clone() {
			absolutize_images(&mut clip.elements, &dir);
		}
		let json = clip.to_json();
		ctx.copy_text(json.clone());
		self.clipboard = Some(json);
		self.notify(format!(
			"Copied {} element{}",
			clip.elements.len(),
			if clip.elements.len() == 1 { "" } else { "s" }
		));
	}

	fn paste(&mut self, text: Option<String>) {
		let text = text.filter(|t| Clip::parse(t).is_some()).or_else(|| self.clipboard.clone());
		let Some(clip) = text.as_deref().and_then(Clip::parse) else { return };
		// Offset when the originals are still here, so the copy is visible.
		let overlaps = clip.elements.iter().any(|e| {
			self.doc.element(&e.base().id).is_some_and(|o| o.base().x == e.base().x && o.base().y == e.base().y)
		});
		let ids = self.doc.insert_copies(&clip.elements, &clip.groups, if overlaps { 20.0 } else { 0.0 }, "Paste");
		self.select_only(ids);
	}

	fn duplicate(&mut self) {
		if self.selection.is_empty() {
			return;
		}
		let clip = Clip::from_selection(&self.doc, &self.selection);
		let ids = self.doc.insert_copies(&clip.elements, &clip.groups, 20.0, "Duplicate");
		for id in &ids {
			if let Some(el) = self.doc.element_mut(id) {
				let label = el.type_label();
				let b = el.base_mut();
				b.name = if b.name.is_empty() { format!("{label} copy") } else { format!("{} copy", b.name) };
			}
		}
		self.select_only(ids);
	}

	fn delete_selection(&mut self) {
		let ids = std::mem::take(&mut self.selection);
		self.doc.delete(&ids);
	}

	fn undo(&mut self) {
		if let Some(name) = self.doc.undo() {
			self.notify(format!("Undid {name}"));
		}
		self.prune_selection();
	}

	fn redo(&mut self) {
		if let Some(name) = self.doc.redo() {
			self.notify(format!("Redid {name}"));
		}
		self.prune_selection();
	}

	/// Moves the selection (unlocked elements only), as one merged undo step per `key`.
	fn nudge(&mut self, dx: f32, dy: f32) {
		let ids: Vec<String> = self.selection.iter().filter(|id| !self.doc.is_locked(id)).cloned().collect();
		if ids.is_empty() {
			return;
		}
		self.doc.checkpoint_merged(format!("Nudge {}", self.selection_label()), "nudge");
		for id in &ids {
			if let Some(el) = self.doc.element_mut(id) {
				geometry::translate(el, dx, dy);
			}
		}
	}

	fn selection_label(&self) -> String {
		match self.selection.as_slice() {
			[one] => self.doc.element(one).map(|e| e.type_label().to_string()).unwrap_or_default(),
			many => format!("{} elements", many.len()),
		}
	}

	fn selected_bounds(&self) -> Vec<(String, Bounds)> {
		self.selection.iter().filter_map(|id| self.doc.element(id).map(|e| (id.clone(), geometry::bounds(e)))).collect()
	}

	/// Aligns the selection. One element (or one group) aligns to the canvas.
	fn align(&mut self, how: Align) {
		let items: Vec<(String, Bounds)> =
			self.selected_bounds().into_iter().filter(|(id, _)| !self.doc.is_locked(id)).collect();
		if items.is_empty() {
			return;
		}
		let union = items.iter().map(|i| i.1).reduce(|a, b| a.union(&b)).unwrap();
		let single = items.len() == 1
			|| self.selection.iter().all(|id| {
				self.doc.group_of(id).is_some() && self.doc.group_of(id) == self.doc.group_of(&self.selection[0])
			});
		let target = if single || how.to_canvas() {
			Bounds::new(0.0, 0.0, self.doc.def.width as f32, self.doc.def.height as f32)
		} else {
			union
		};
		self.doc.checkpoint(how.name());
		let group_move = single && items.len() > 1;
		for (id, b) in &items {
			// A group (or a single element) moves as one block.
			let b = if group_move { &union } else { b };
			let (dx, dy) = match how {
				Align::Left => (target.left - b.left, 0.0),
				Align::CenterH | Align::CanvasH => (target.center_x() - b.center_x(), 0.0),
				Align::Right => (target.right - b.right, 0.0),
				Align::Top => (0.0, target.top - b.top),
				Align::CenterV | Align::CanvasV => (0.0, target.center_y() - b.center_y()),
				Align::Bottom => (0.0, target.bottom - b.bottom),
				Align::Canvas => (target.center_x() - b.center_x(), target.center_y() - b.center_y()),
			};
			if let Some(el) = self.doc.element_mut(id) {
				geometry::translate(el, dx, dy);
			}
		}
	}

	/// Spaces three or more elements evenly between the outermost two.
	fn distribute(&mut self, horizontal: bool) {
		let mut items: Vec<(String, Bounds)> =
			self.selected_bounds().into_iter().filter(|(id, _)| !self.doc.is_locked(id)).collect();
		if items.len() < 3 {
			self.notify("Select three or more elements to distribute");
			return;
		}
		let center = |b: &Bounds| if horizontal { b.center_x() } else { b.center_y() };
		items.sort_by(|a, b| center(&a.1).total_cmp(&center(&b.1)));
		let first = center(&items[0].1);
		let step = (center(&items[items.len() - 1].1) - first) / (items.len() - 1) as f32;
		self.doc.checkpoint(if horizontal { "Distribute horizontally" } else { "Distribute vertically" });
		for (i, (id, b)) in items.iter().enumerate() {
			let d = first + step * i as f32 - center(b);
			if let Some(el) = self.doc.element_mut(id) {
				if horizontal {
					geometry::translate(el, d, 0.0);
				} else {
					geometry::translate(el, 0.0, d);
				}
			}
		}
	}

	// ── Rendering ──

	fn ensure_rendered(&mut self, ctx: &egui::Context) {
		if self.animate {
			let t = self.anim_start.elapsed().as_secs_f32();
			let phase = (t / 4.0).fract();
			let tri = if phase < 0.5 { phase * 2.0 } else { 2.0 - phase * 2.0 };
			for (name, min, max) in self.doc.bound_sources() {
				self.values.insert(name, min + (max - min) * tri);
			}
			self.values_revision += 1;
			ctx.request_repaint();
		}
		// Clocks tick once a second.
		let has_clock = self.doc.def.elements.iter().any(|e| matches!(e, Element::Clock(_)));
		let second = chrono::Local::now().timestamp();
		if has_clock && second != self.last_clock_second {
			self.last_clock_second = second;
			self.values_revision += 1;
		}
		if has_clock {
			ctx.request_repaint_after(Duration::from_millis(250));
		}

		let key = (self.doc.revision, self.values_revision, self.preview);
		if self.rendered == Some(key) && self.texture.is_some() {
			return;
		}
		let def = self.doc.preview_definition();
		let (w, h) = (def.width.max(1) as u32, def.height.max(1) as u32);
		if self.canvas.width() != w || self.canvas.height() != h {
			self.canvas = Canvas::new(w, h);
		}
		self.renderer.render(&mut self.canvas, &def, &self.values, None);
		preview::apply(&mut self.canvas, &self.preview);
		let image = egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], self.canvas.pixmap().data());
		let filter = self.texture_options();
		match &mut self.texture {
			Some(t) => t.set(image, filter),
			None => self.texture = Some(ctx.load_texture("gauge", image, filter)),
		}
		self.texture_filter = Some(filter);
		self.rendered = Some(key);
	}

	/// Crisp pixels when zoomed in, smooth when zoomed out.
	fn texture_options(&self) -> TextureOptions {
		if self.zoom >= 1.0 {
			TextureOptions::NEAREST
		} else {
			TextureOptions::LINEAR
		}
	}

	fn refresh_analysis(&mut self) {
		if self.issues_revision != self.doc.revision {
			self.issues = validate::validate(&self.doc.def);
			self.issues_revision = self.doc.revision;
		}
	}

	fn measure_cost(&mut self) {
		let def = self.doc.preview_definition();
		self.cost = Some(cost::measure(&def, &self.values));
		self.cost_revision = self.doc.revision;
	}

	fn poll_jobs(&mut self, ctx: &egui::Context) {
		if self.export_job.as_ref().is_some_and(|j| j.handle.is_finished()) {
			let job = self.export_job.take().unwrap();
			match job.handle.join() {
				Ok(Ok(path)) => self.notify(format!("Exported {}", path.display())),
				Ok(Err(e)) => self.error(format!("Export failed: {e}")),
				Err(_) => self.error("Export failed"),
			}
		}
		if self.deploy_job.as_ref().is_some_and(|j| j.handle.is_finished()) {
			let job = self.deploy_job.take().unwrap();
			match job.handle.join() {
				Ok(Ok((log, config_path))) => {
					self.deploy_log =
						format!("{log}\nDone. Point a Custom screen's CustomDefinitionFile at \"{config_path}\".");
					self.notify("Uploaded to the device");
				}
				Ok(Err((log, e))) => {
					self.deploy_log = format!("{log}\n{e}");
					self.error(format!("Deploy failed: {e}"));
				}
				Err(_) => self.error("Deploy failed"),
			}
		}
		if self.export_job.is_some() || self.deploy_job.is_some() {
			ctx.request_repaint_after(Duration::from_millis(100));
		}
	}

	// ── UI ──

	fn shortcuts(&mut self, ctx: &egui::Context) {
		let typing = ctx.text_edit_focused();
		let cmd = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, k));
		let cmd_shift = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, k));
		if cmd(Key::S) {
			self.save(false);
		}
		if cmd_shift(Key::S) {
			self.save(true);
		}
		if cmd(Key::O) {
			self.guard(Pending::Open(None));
		}
		if cmd(Key::N) {
			self.guard(Pending::New(1));
		}
		if typing {
			return;
		}
		if cmd_shift(Key::Z) || cmd(Key::Y) {
			self.redo();
		}
		if cmd(Key::Z) {
			self.undo();
		}
		if cmd(Key::D) {
			self.duplicate();
		}
		if cmd(Key::A) {
			self.selection = self
				.doc
				.def
				.elements
				.iter()
				.map(|e| e.base().id.clone())
				.filter(|id| !self.doc.is_hidden(id))
				.collect();
		}
		if cmd_shift(Key::G) {
			let s = self.selection.clone();
			self.doc.ungroup(&s);
		}
		if cmd(Key::G) {
			let s = self.selection.clone();
			self.doc.group(&s);
		}
		if cmd(Key::L) {
			let s = self.selection.clone();
			let lock = !s.iter().all(|id| self.doc.is_locked(id));
			self.doc.set_locked(&s, lock);
		}
		if cmd(Key::H) {
			let s = self.selection.clone();
			let hide = !s.iter().all(|id| self.doc.is_hidden(id));
			self.doc.set_hidden(&s, hide);
		}
		if cmd(Key::Num0) {
			self.fit_pending = true;
		}
		if cmd(Key::Num1) {
			self.zoom = 1.0;
			self.pan = Vec2::ZERO;
		}
		let (events, plain, shift) = ctx.input(|i| (i.events.clone(), i.modifiers.is_none(), i.modifiers.shift_only()));
		for event in events {
			match event {
				egui::Event::Copy => self.copy(ctx),
				egui::Event::Cut => {
					self.copy(ctx);
					self.delete_selection();
				}
				egui::Event::Paste(text) => self.paste(Some(text)),
				_ => {}
			}
		}
		if !(plain || shift) {
			return;
		}
		let step = if shift { 10.0 } else { 1.0 };
		let pressed = |k: Key| ctx.input(|i| i.key_pressed(k));
		if pressed(Key::Delete) || pressed(Key::Backspace) {
			self.delete_selection();
		}
		if pressed(Key::Escape) {
			self.selection.clear();
		}
		if plain && pressed(Key::G) {
			self.snap_to_grid = !self.snap_to_grid;
			self.notify(if self.snap_to_grid { "Grid snap on" } else { "Grid snap off" });
		}
		let (mut dx, mut dy) = (0.0, 0.0);
		if pressed(Key::ArrowLeft) {
			dx -= step;
		}
		if pressed(Key::ArrowRight) {
			dx += step;
		}
		if pressed(Key::ArrowUp) {
			dy -= step;
		}
		if pressed(Key::ArrowDown) {
			dy += step;
		}
		if dx != 0.0 || dy != 0.0 {
			self.nudge(dx, dy);
		}
	}

	fn menu_bar(&mut self, ui: &mut Ui) {
		egui::MenuBar::new().ui(ui, |ui| {
			ui.menu_button("File", |ui| {
				ui.menu_button("New", |ui| {
					for (i, (name, _)) in crate::templates::TEMPLATES.iter().enumerate() {
						if ui.button(*name).clicked() {
							self.guard(Pending::New(i));
						}
					}
				});
				if ui.button("Open…  (Ctrl+O)").clicked() {
					self.guard(Pending::Open(None));
				}
				if ui.button("Save  (Ctrl+S)").clicked() {
					self.save(false);
				}
				if ui.button("Save as…").clicked() {
					self.save(true);
				}
				ui.separator();
				if ui.button("Insert snippet…").clicked() {
					self.insert_snippet();
				}
				if ui.add_enabled(!self.selection.is_empty(), egui::Button::new("Save selection as snippet…")).clicked()
				{
					self.save_snippet();
				}
				ui.separator();
				if ui.button("Export PNG…").clicked() {
					self.export_png();
				}
				if ui.add_enabled(self.export_job.is_none(), egui::Button::new("Export animated GIF…")).clicked() {
					self.export_gif();
				}
				ui.separator();
				if ui.button("Screens (gauges.json)…").clicked() && self.config.is_none() {
					self.config = Some(ConfigEditor::empty());
				}
				if ui.button("Deploy to device…").clicked() {
					self.deploy_open = true;
				}
				ui.separator();
				if ui.button("Quit").clicked() {
					ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
				}
			});
			ui.menu_button("Edit", |ui| {
				let undo = self.doc.undo_name().map(|n| format!("Undo {n}")).unwrap_or_else(|| "Undo".into());
				if ui
					.add_enabled(self.doc.undo_name().is_some(), egui::Button::new(undo).shortcut_text("Ctrl+Z"))
					.clicked()
				{
					self.undo();
				}
				let redo = self.doc.redo_name().map(|n| format!("Redo {n}")).unwrap_or_else(|| "Redo".into());
				if ui
					.add_enabled(self.doc.redo_name().is_some(), egui::Button::new(redo).shortcut_text("Ctrl+Y"))
					.clicked()
				{
					self.redo();
				}
				ui.checkbox(&mut self.show_history, "Show history");
				ui.separator();
				let any = !self.selection.is_empty();
				if ui.add_enabled(any, egui::Button::new("Copy").shortcut_text("Ctrl+C")).clicked() {
					self.copy(ui.ctx());
				}
				if ui.add_enabled(any, egui::Button::new("Cut").shortcut_text("Ctrl+X")).clicked() {
					self.copy(ui.ctx());
					self.delete_selection();
				}
				if ui.add(egui::Button::new("Paste").shortcut_text("Ctrl+V")).clicked() {
					self.paste(None);
				}
				if ui.add_enabled(any, egui::Button::new("Duplicate").shortcut_text("Ctrl+D")).clicked() {
					self.duplicate();
				}
				if ui.add_enabled(any, egui::Button::new("Delete").shortcut_text("Del")).clicked() {
					self.delete_selection();
				}
				ui.separator();
				let s = self.selection.clone();
				if ui.add_enabled(s.len() > 1, egui::Button::new("Group").shortcut_text("Ctrl+G")).clicked() {
					self.doc.group(&s);
				}
				if ui.add_enabled(any, egui::Button::new("Ungroup").shortcut_text("Ctrl+Shift+G")).clicked() {
					self.doc.ungroup(&s);
				}
				if ui.add_enabled(any, egui::Button::new("Lock / unlock").shortcut_text("Ctrl+L")).clicked() {
					let lock = !s.iter().all(|id| self.doc.is_locked(id));
					self.doc.set_locked(&s, lock);
				}
				if ui.add_enabled(any, egui::Button::new("Hide / show").shortcut_text("Ctrl+H")).clicked() {
					let hide = !s.iter().all(|id| self.doc.is_hidden(id));
					self.doc.set_hidden(&s, hide);
				}
			});
			ui.menu_button("Arrange", |ui| {
				let any = !self.selection.is_empty();
				ui.add_enabled_ui(any, |ui| {
					for how in Align::ALL {
						if ui.button(how.name()).clicked() {
							self.align(how);
						}
					}
					ui.separator();
					if ui.button("Distribute horizontally").clicked() {
						self.distribute(true);
					}
					if ui.button("Distribute vertically").clicked() {
						self.distribute(false);
					}
					ui.separator();
					let s = self.selection.clone();
					if ui.button("Bring to front").clicked() {
						self.doc.move_to_end(&s, true);
					}
					if ui.button("Bring forward").clicked() {
						self.doc.move_in_order(&s, 1);
					}
					if ui.button("Send backward").clicked() {
						self.doc.move_in_order(&s, -1);
					}
					if ui.button("Send to back").clicked() {
						self.doc.move_to_end(&s, false);
					}
				});
			});
			ui.menu_button("View", |ui| {
				if ui.add(egui::Button::new("Fit").shortcut_text("Ctrl+0")).clicked() {
					self.fit_pending = true;
				}
				if ui.add(egui::Button::new("Actual size").shortcut_text("Ctrl+1")).clicked() {
					self.zoom = 1.0;
					self.pan = Vec2::ZERO;
				}
				if ui.button("Zoom 2×").clicked() {
					self.zoom = 2.0;
				}
				ui.separator();
				ui.checkbox(&mut self.show_grid, "Show grid");
				ui.checkbox(&mut self.snap_to_grid, "Snap to grid (G)");
				ui.horizontal(|ui| {
					ui.label("Grid size");
					ui.add(egui::DragValue::new(&mut self.grid_size).range(1.0..=200.0).suffix(" px"));
				});
				ui.checkbox(&mut self.smart_guides, "Smart guides (hold Alt to bypass)");
				ui.checkbox(&mut self.rulers, "Rulers");
				ui.separator();
				ui.label(RichText::new("Device preview").strong());
				ui.checkbox(&mut self.preview.fps, "FPS counter");
				let mut battery = self.preview.battery.is_some();
				ui.horizontal(|ui| {
					ui.checkbox(&mut battery, "Low battery");
					let mut pct = self.preview.battery.unwrap_or(20);
					ui.add_enabled(battery, egui::Slider::new(&mut pct, 0..=30).suffix(" %"));
					self.preview.battery = battery.then_some(pct);
				});
				ui.checkbox(&mut self.preview.lcd, "TurboXs LCD look (approximate)");
			});
		});
	}

	fn toolbox(&mut self, ui: &mut Ui) {
		ui.horizontal_wrapped(|ui| {
			if ui
				.add_enabled(self.doc.undo_name().is_some(), egui::Button::new("Undo"))
				.on_hover_text(self.doc.undo_name().map(|n| format!("Undo {n}")).unwrap_or_default())
				.clicked()
			{
				self.undo();
			}
			if ui
				.add_enabled(self.doc.redo_name().is_some(), egui::Button::new("Redo"))
				.on_hover_text(self.doc.redo_name().map(|n| format!("Redo {n}")).unwrap_or_default())
				.clicked()
			{
				self.redo();
			}
			ui.toggle_value(&mut self.snap_to_grid, "Snap (G)");
			ui.separator();
			ui.label(RichText::new("Add").strong());
			for (tag, label) in TYPES {
				if ui.button(label).clicked() {
					self.add_element(tag);
				}
			}
		});
	}

	fn element_list(&mut self, ui: &mut Ui) {
		ui.horizontal(|ui| {
			ui.heading("Elements");
		});
		let s = self.selection.clone();
		let any = !s.is_empty();
		ui.horizontal_wrapped(|ui| {
			if ui.add_enabled(any, egui::Button::new("⬆")).on_hover_text("Send backward (draw earlier)").clicked() {
				self.doc.move_in_order(&s, -1);
			}
			if ui.add_enabled(any, egui::Button::new("⬇")).on_hover_text("Bring forward (draw later)").clicked() {
				self.doc.move_in_order(&s, 1);
			}
			if ui.add_enabled(any, egui::Button::new("Duplicate")).clicked() {
				self.duplicate();
			}
			if ui.add_enabled(any, egui::Button::new("Delete")).clicked() {
				self.delete_selection();
			}
			if ui.add_enabled(s.len() > 1, egui::Button::new("Group")).clicked() {
				self.doc.group(&s);
			}
			if ui.add_enabled(any, egui::Button::new("Ungroup")).clicked() {
				self.doc.ungroup(&s);
			}
		});
		ui.separator();
		egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
			if ui.selectable_label(self.selection.is_empty(), "[Canvas] Background").clicked() {
				self.selection.clear();
			}
			let rows: Vec<(String, String, bool, bool, Option<u32>)> = self
				.doc
				.def
				.elements
				.iter()
				.map(|e| {
					let id = e.base().id.clone();
					let label = format!("[{}] {}", e.type_label(), list_name(e));
					(id.clone(), label, self.doc.is_hidden(&id), self.doc.is_locked(&id), self.doc.group_of(&id))
				})
				.collect();
			let mut toggle_hidden = None;
			let mut toggle_locked = None;
			for (index, (id, label, hidden, locked, group)) in rows.iter().enumerate() {
				ui.horizontal(|ui| {
					if ui
						.selectable_label(false, if *hidden { "◌" } else { "👁" })
						.on_hover_text(if *hidden {
							"Hidden in the preview (still on the device). Click to show."
						} else {
							"Hide in the preview"
						})
						.clicked()
					{
						toggle_hidden = Some((id.clone(), !hidden));
					}
					if ui
						.selectable_label(false, if *locked { "🔒" } else { "🔓" })
						.on_hover_text(if *locked {
							"Locked. Click to unlock."
						} else {
							"Lock (can't be picked or moved on the canvas)"
						})
						.clicked()
					{
						toggle_locked = Some((id.clone(), !locked));
					}
					let mut text = RichText::new(match group {
						Some(g) => format!("{label}  ⧉{g}"),
						None => label.clone(),
					});
					if *hidden {
						text = text.weak();
					}
					let selected = self.selection.contains(id);
					let resp = ui.selectable_label(selected, text);
					if resp.clicked() {
						let mods = ui.input(|i| i.modifiers);
						if mods.command {
							if selected {
								self.selection.retain(|s| s != id);
							} else {
								self.selection.push(id.clone());
							}
						} else if mods.shift && !self.selection.is_empty() {
							let anchor = self.primary().and_then(|p| self.doc.index_of(p)).unwrap_or(index);
							let (a, b) = (anchor.min(index), anchor.max(index));
							self.selection = rows[a..=b].iter().map(|r| r.0.clone()).collect();
						} else {
							self.selection = vec![id.clone()];
						}
					}
				});
			}
			if let Some((id, hide)) = toggle_hidden {
				self.doc.set_hidden(&[id], hide);
			}
			if let Some((id, lock)) = toggle_locked {
				self.doc.set_locked(&[id], lock);
			}
		});
	}

	fn source_names(&self) -> Vec<String> {
		let mut names: Vec<String> = me_data::DATA_SOURCE_NAMES.iter().map(|s| s.to_string()).collect();
		names.extend(self.doc.def.calculated_channels.iter().filter(|c| !c.name.is_empty()).map(|c| c.name.clone()));
		names
	}

	fn properties(&mut self, ui: &mut Ui) {
		let sources = self.source_names();
		let start_dir = self.doc.def.base_directory.clone().or_else(|| self.last_dir.clone());
		let ctx = PropContext { sources: &sources, start_dir: start_dir.as_deref() };

		if self.selection.len() > 1 {
			ui.label(RichText::new(format!("{} elements selected", self.selection.len())).strong());
			ui.label("Drag to move them together, or use Arrange to line them up. Properties of the last one picked:");
			ui.add_space(4.0);
		}
		let Some(id) = self.primary().map(str::to_string) else {
			ui.heading("Canvas");
			let mut value = serde_json::to_value(&self.doc.def).unwrap_or_default();
			let obj = value.as_object_mut().unwrap();
			if let Some(field) = props::edit_fields(ui, "canvas", fields::CANVAS, obj, &ctx) {
				if let Ok(mut def) = serde_json::from_value::<CustomGaugeDefinition>(value) {
					self.doc.checkpoint_merged(format!("Change {}", field.label), &format!("canvas/{}", field.key));
					def.base_directory = self.doc.def.base_directory.clone();
					self.doc.def = def;
				}
			}
			ui.add_space(8.0);
			self.calculated_channels(ui);
			return;
		};
		let Some(element) = self.doc.element(&id) else { return };
		let mut value = serde_json::to_value(element).unwrap_or_default();
		let tag = fields::type_tag(&value).to_string();
		ui.horizontal(|ui| {
			ui.heading(format!("{} properties", element.type_label()));
			if self.doc.is_locked(&id) {
				ui.label("🔒");
			}
		});
		let fields = fields::fields_for(&tag);
		let changed = props::edit_fields(ui, &id, &fields, value.as_object_mut().unwrap(), &ctx);
		if let Some(field) = changed {
			match serde_json::from_value::<Element>(value) {
				Ok(new) => {
					let label = describe(&new);
					self.doc.checkpoint_merged(
						format!("Change {} of {label}", field.label),
						&format!("{id}/{}", field.key),
					);
					if let Some(el) = self.doc.element_mut(&id) {
						*el = new;
					}
				}
				Err(e) => self.error(format!("{}: {e}", field.label)),
			}
		}
		ui.add_space(6.0);
		ui.label(RichText::new(format!("Id {id}")).weak().small());
	}

	fn calculated_channels(&mut self, ui: &mut Ui) {
		ui.separator();
		ui.label(RichText::new("Calculated channels").strong());
		let mut remove = None;
		let mut edit: Option<(usize, String, String)> = None;
		for (i, ch) in self.doc.def.calculated_channels.iter().enumerate() {
			ui.horizontal(|ui| {
				let (mut name, mut expr) = (ch.name.clone(), ch.expression.clone());
				let n = ui.add(egui::TextEdit::singleline(&mut name).hint_text("Name").desired_width(90.0)).changed();
				let mut expr_edit = egui::TextEdit::singleline(&mut expr)
					.hint_text("Expression, e.g. Rpm / 1000")
					.desired_width(ui.available_width() - 30.0);
				let bad = validate::check_expression(&ch.expression).err();
				if bad.is_some() {
					expr_edit = expr_edit.text_color(props::ERROR);
				}
				let mut resp = ui.add(expr_edit);
				if let Some(b) = bad {
					resp = resp.on_hover_text(b);
				}
				if n || resp.changed() {
					edit = Some((i, name, expr));
				}
				if ui.button("✖").clicked() {
					remove = Some(i);
				}
			});
		}
		if let Some((i, name, expr)) = edit {
			self.doc.checkpoint_merged("Edit calculated channel", &format!("channel/{i}"));
			self.doc.def.calculated_channels[i] = CalculatedChannel { name, expression: expr };
		}
		if let Some(i) = remove {
			self.doc.checkpoint("Remove calculated channel");
			self.doc.def.calculated_channels.remove(i);
		}
		if ui.button("+ Add channel").clicked() {
			self.doc.checkpoint("Add calculated channel");
			self.doc.def.calculated_channels.push(CalculatedChannel::default());
		}
	}

	fn test_values(&mut self, ui: &mut Ui) {
		let sources = self.doc.bound_sources();
		ui.horizontal(|ui| {
			if ui
				.toggle_value(&mut self.animate, "▶ Animate")
				.on_hover_text("Sweep every value from min to max")
				.changed()
			{
				self.anim_start = Instant::now();
			}
			if ui.button("Reset").clicked() {
				self.values.clear();
				self.animate = false;
				self.rerender_values();
			}
		});
		if sources.is_empty() {
			ui.label(RichText::new("No data-bound elements").italics().weak());
		}
		for (name, min, max) in sources {
			let v = self.values.entry(name.clone()).or_insert(min);
			let (lo, hi) = if max > min { (min, max) } else { (min, min + 1.0) };
			ui.horizontal(|ui| {
				ui.add_sized([110.0, 18.0], egui::Label::new(&name).truncate());
				if ui.add_enabled(!self.animate, egui::Slider::new(v, lo..=hi).max_decimals(2)).changed() {
					self.values_revision += 1;
				}
			});
		}
	}

	fn validation(&mut self, ui: &mut Ui) {
		if self.issues.is_empty() {
			ui.label(RichText::new("✔ No problems found").color(Color32::from_rgb(0x60, 0xD0, 0x80)));
			return;
		}
		let mut select = None;
		for issue in &self.issues {
			let (icon, color) = match issue.severity {
				Severity::Error => ("⛔", props::ERROR),
				Severity::Warning => ("⚠", props::WARN),
				Severity::Info => ("ℹ", ui.visuals().weak_text_color()),
			};
			let resp = ui.add(
				egui::Label::new(RichText::new(format!("{icon} {}", issue.message)).color(color))
					.wrap()
					.sense(Sense::click()),
			);
			if resp.clicked() {
				select = Some(issue.element.clone());
			}
			if issue.element.is_some() {
				resp.on_hover_cursor(CursorIcon::PointingHand);
			}
		}
		if let Some(target) = select {
			self.selection = target.into_iter().collect();
		}
	}

	fn cost_panel(&mut self, ui: &mut Ui) {
		ui.horizontal(|ui| {
			if ui.button("Measure").clicked() {
				self.measure_cost();
			}
			ui.checkbox(&mut self.auto_cost, "After each change");
		});
		if cfg!(debug_assertions) {
			ui.label(
				RichText::new("Debug build: times are several times too high. Run with --release.")
					.small()
					.color(props::WARN),
			);
		}
		let Some(report) = &self.cost else {
			ui.label(RichText::new("Times each element with the device renderer.").weak());
			return;
		};
		let stale = self.cost_revision != self.doc.revision;
		let budget = cost::FRAME_BUDGET_MS;
		let line = |ui: &mut Ui, label: &str, ms: f64| {
			let device = ms * cost::DEVICE_FACTOR;
			let color = if device > budget {
				props::ERROR
			} else if device > budget / 2.0 {
				props::WARN
			} else {
				ui.visuals().text_color()
			};
			ui.label(RichText::new(format!("{label}: {ms:.2} ms here, ≈ {device:.1} ms on the RG35XX")).color(color));
		};
		line(ui, "Full repaint", report.full_ms());
		line(ui, "Per frame while values move", report.dynamic_ms());
		ui.label(
			RichText::new(format!(
				"Device estimate = ×{} (bench/RESULTS.md). 60 fps budget: {budget:.1} ms.",
				cost::DEVICE_FACTOR
			))
			.small()
			.weak(),
		);
		if stale {
			ui.label(RichText::new("Changed since measured.").small().color(props::WARN));
		}
		let mut rows: Vec<(String, f64, bool)> = report.elements.clone();
		rows.sort_by(|a, b| b.1.total_cmp(&a.1));
		let max = report.max_ms().max(1e-6);
		let mut select = None;
		egui::Grid::new("cost").num_columns(3).spacing([6.0, 2.0]).show(ui, |ui| {
			ui.label("Background");
			ui.label(format!("{:.3}", report.backdrop_ms));
			bar(ui, (report.backdrop_ms / max) as f32);
			ui.end_row();
			for (id, ms, dynamic) in rows {
				let name = self
					.doc
					.element(&id)
					.map(|e| format!("{} {}", e.type_label(), list_name(e)))
					.unwrap_or_else(|| id.clone());
				let text = if dynamic { RichText::new(name) } else { RichText::new(name).weak() };
				if ui
					.add(egui::Label::new(text).truncate().sense(Sense::click()))
					.on_hover_text(if dynamic {
						"Redrawn when its value changes"
					} else {
						"Static: cached, only drawn on a full repaint"
					})
					.clicked()
				{
					select = Some(id.clone());
				}
				ui.label(format!("{ms:.3}"));
				bar(ui, (ms / max) as f32);
				ui.end_row();
			}
		});
		if let Some(id) = select {
			self.selection = vec![id];
		}
	}

	fn deploy_window(&mut self, ctx: &egui::Context) {
		let mut open = self.deploy_open;
		egui::Window::new("Deploy to device").open(&mut open).resizable(true).default_width(460.0).show(ctx, |ui| {
			egui::Grid::new("deploy").num_columns(2).show(ui, |ui| {
				ui.label("Host");
				ui.text_edit_singleline(&mut self.deploy_target.host);
				ui.end_row();
				ui.label("User");
				ui.text_edit_singleline(&mut self.deploy_target.user);
				ui.end_row();
				ui.label("Password");
				ui.add(egui::TextEdit::singleline(&mut self.deploy_target.password).password(true))
					.on_hover_text("Used through sshpass when it is installed; otherwise ssh uses your key");
				ui.end_row();
				ui.label("App folder");
				ui.text_edit_singleline(&mut self.deploy_target.bin_dir);
				ui.end_row();
				ui.label("Subfolder");
				ui.add(
					egui::TextEdit::singleline(&mut self.deploy_subdir)
						.hint_text("e.g. gauges/rpm (empty = app folder)"),
				);
				ui.end_row();
			});
			ui.label(
				RichText::new(
					"Defaults come from REMOTE_HOST, REMOTE_USER, REMOTE_PASS and BIN_DIR, as in deploy-arm64.sh.",
				)
				.small()
				.weak(),
			);
			ui.horizontal(|ui| {
				let busy = self.deploy_job.is_some();
				if ui.add_enabled(!busy, egui::Button::new("Save and upload this gauge")).clicked()
					&& (self.doc.path.is_some() || self.save(false))
				{
					if self.doc.is_dirty() && !self.save(false) {
						return;
					}
					let path = self.doc.path.clone().unwrap();
					let images = deploy::packaged_images(&self.doc.def, &path);
					let target = self.deploy_target.clone();
					let subdir = self.deploy_subdir.clone();
					self.deploy_log = format!("Uploading {} and {} image(s)…\n", path.display(), images.len());
					let handle = std::thread::spawn(move || target.upload_gauge(&path, &images, &subdir));
					self.deploy_job = Some(Job { label: "Uploading…".into(), handle });
				}
				if let Some(job) = &self.deploy_job {
					ui.spinner();
					ui.label(&job.label);
				}
			});
			ui.label(
				RichText::new("The device app reads files at start-up: restart it to see the change.").small().weak(),
			);
			egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
				ui.add(
					egui::TextEdit::multiline(&mut self.deploy_log.as_str())
						.font(egui::TextStyle::Monospace)
						.desired_width(f32::INFINITY),
				);
			});
		});
		self.deploy_open = open;
	}

	fn history_window(&mut self, ctx: &egui::Context) {
		let mut open = self.show_history;
		egui::Window::new("History").open(&mut open).default_width(260.0).show(ctx, |ui| {
			egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
				let names: Vec<String> = self.doc.history_names().map(str::to_string).collect();
				if names.is_empty() {
					ui.label(RichText::new("Nothing to undo").weak());
				}
				for name in names {
					ui.label(name);
				}
				if let Some(redo) = self.doc.redo_name() {
					ui.label(RichText::new(format!("(redo) {redo}")).weak());
				}
			});
		});
		self.show_history = open;
	}

	fn pending_modal(&mut self, ctx: &egui::Context) {
		let Some(action) = self.pending.clone() else { return };
		let mut choice = None;
		egui::Modal::new(egui::Id::new("unsaved")).show(ctx, |ui| {
			ui.heading("Unsaved changes");
			ui.label(format!(
				"Save the changes to {}?",
				self.doc.path.as_deref().map(|p| p.display().to_string()).unwrap_or_else(|| "this gauge".into())
			));
			ui.horizontal(|ui| {
				if ui.button("Save").clicked() {
					choice = Some(0);
				}
				if ui.button("Don't save").clicked() {
					choice = Some(1);
				}
				if ui.button("Cancel").clicked() {
					choice = Some(2);
				}
			});
		});
		match choice {
			Some(0) => {
				self.pending = None;
				if self.save(false) {
					self.finish_pending(ctx, action);
				}
			}
			Some(1) => {
				self.pending = None;
				self.finish_pending(ctx, action);
			}
			Some(_) => self.pending = None,
			None => {}
		}
	}

	fn finish_pending(&mut self, ctx: &egui::Context, action: Pending) {
		let quit = matches!(action, Pending::Quit);
		self.run_pending(action);
		if quit {
			ctx.send_viewport_cmd(egui::ViewportCommand::Close);
		}
	}

	fn status_bar(&mut self, ui: &mut Ui) {
		ui.horizontal(|ui| {
			ui.label(self.doc.title());
			ui.separator();
			ui.label(format!("{:.0} %", self.zoom * 100.0));
			if let Some(p) = self.cursor {
				ui.separator();
				ui.label(format!("{:.0}, {:.0}", p.x, p.y));
			}
			if !self.selection.is_empty() {
				ui.separator();
				ui.label(format!("{} selected", self.selection.len()));
			}
			if let Some(job) = &self.export_job {
				ui.separator();
				ui.spinner();
				ui.label(&job.label);
			}
			if let Some((text, at, error)) = &self.message {
				if at.elapsed() < Duration::from_secs(6) {
					ui.separator();
					ui.label(if *error { RichText::new(text).color(props::ERROR) } else { RichText::new(text) });
				}
			}
		});
	}

	// ── Canvas ──

	fn canvas_view(&mut self, ui: &mut Ui) {
		let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
		let full = resp.rect;
		let view = if self.rulers { Rect::from_min_max(full.min + vec2(RULER, RULER), full.max) } else { full };
		let (w, h) = (self.doc.def.width.max(1) as f32, self.doc.def.height.max(1) as f32);
		if self.fit_pending && view.width() > 50.0 {
			self.zoom = ((view.width() - 40.0) / w).min((view.height() - 40.0) / h).clamp(0.1, 8.0);
			self.pan = Vec2::ZERO;
			self.fit_pending = false;
		}

		// Zoom (Ctrl+wheel, pinch) about the pointer, and pan (wheel, middle drag, Space+drag).
		let hover = ui.input(|i| i.pointer.hover_pos()).filter(|p| view.contains(*p));
		if let Some(pos) = hover {
			let (zoom_delta, scroll) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta));
			if zoom_delta != 1.0 {
				let before = self.to_canvas(view, pos);
				self.zoom = (self.zoom * zoom_delta).clamp(0.1, 16.0);
				let after = self.to_screen(view, before);
				self.pan += pos - after;
			}
			self.pan += scroll;
		}
		if ui.input(|i| i.pointer.button_down(PointerButton::Middle))
			&& (hover.is_some() || matches!(self.interaction, Interaction::Pan))
		{
			self.pan += ui.input(|i| i.pointer.delta());
		}

		let origin = self.to_screen(view, Pos2::ZERO);
		let image_rect = Rect::from_min_size(origin, vec2(w, h) * self.zoom);
		let clip = painter.with_clip_rect(view);
		clip.rect_filled(view, 0.0, Color32::from_gray(0x18));
		if let Some(tex) = &self.texture {
			if self.texture_filter != Some(self.texture_options()) {
				self.rendered = None; // re-upload with the other filter next frame
				ui.ctx().request_repaint();
			}
			clip.image(tex.id(), image_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
		}
		clip.rect_stroke(image_rect, 0.0, Stroke::new(1.0, Color32::from_gray(0x55)), egui::StrokeKind::Outside);

		if self.show_grid && self.grid_size * self.zoom >= 4.0 {
			let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 22));
			let mut x = 0.0;
			while x <= w {
				let sx = origin.x + x * self.zoom;
				clip.line_segment([pos2(sx, image_rect.top()), pos2(sx, image_rect.bottom())], stroke);
				x += self.grid_size;
			}
			let mut y = 0.0;
			while y <= h {
				let sy = origin.y + y * self.zoom;
				clip.line_segment([pos2(image_rect.left(), sy), pos2(image_rect.right(), sy)], stroke);
				y += self.grid_size;
			}
		}

		self.handle_pointer(ui, view, &resp);
		self.draw_overlay(&clip, view);
		if self.rulers {
			self.draw_rulers(&painter, full, view);
		}
	}

	fn to_screen(&self, view: Rect, p: Pos2) -> Pos2 {
		let (w, h) = (self.doc.def.width as f32, self.doc.def.height as f32);
		let origin = view.center() - vec2(w, h) * self.zoom / 2.0 + self.pan;
		origin + p.to_vec2() * self.zoom
	}

	fn to_canvas(&self, view: Rect, s: Pos2) -> Pos2 {
		let (w, h) = (self.doc.def.width as f32, self.doc.def.height as f32);
		let origin = view.center() - vec2(w, h) * self.zoom / 2.0 + self.pan;
		((s - origin) / self.zoom).to_pos2()
	}

	fn snap(&self, v: f32, alt: bool) -> f32 {
		if self.snap_to_grid && !alt {
			(v / self.grid_size).round() * self.grid_size
		} else {
			v
		}
	}

	/// Topmost visible, unlocked element under `p`.
	fn pick(&self, p: Pos2) -> Option<String> {
		self.doc
			.def
			.elements
			.iter()
			.rev()
			.find(|e| {
				let id = &e.base().id;
				!self.doc.is_hidden(id) && !self.doc.is_locked(id) && geometry::hit_test(e, p.x, p.y)
			})
			.map(|e| e.base().id.clone())
	}

	/// The outline of an element on screen (6 canvas px outside its bounds, as in C#).
	fn outline(&self, view: Rect, b: Bounds) -> Rect {
		let b = b.inflate(6.0);
		Rect::from_min_max(self.to_screen(view, pos2(b.left, b.top)), self.to_screen(view, pos2(b.right, b.bottom)))
	}

	fn single_editable(&self) -> Option<(String, Element)> {
		let [id] = self.selection.as_slice() else { return None };
		if self.doc.is_locked(id) {
			return None;
		}
		self.doc.element(id).map(|e| (id.clone(), e.clone()))
	}

	fn handle_pointer(&mut self, ui: &mut Ui, view: Rect, resp: &egui::Response) {
		let (pos, pressed, down, released, mods, space) = ui.input(|i| {
			(
				i.pointer.interact_pos(),
				i.pointer.primary_pressed(),
				i.pointer.primary_down(),
				i.pointer.primary_released(),
				i.modifiers,
				i.key_down(Key::Space),
			)
		});
		self.cursor =
			ui.input(|i| i.pointer.hover_pos()).filter(|p| view.contains(*p)).map(|p| self.to_canvas(view, p));
		let Some(pos) = pos else { return };
		let p = self.to_canvas(view, pos);

		if pressed && resp.hovered() && view.contains(pos) {
			self.interaction = self.start_interaction(view, pos, p, mods, space);
		}

		if down {
			if matches!(self.interaction, Interaction::Pan) {
				self.pan += ui.input(|i| i.pointer.delta());
			}
			self.update_interaction(view, p, mods.alt, mods.shift);
		} else if !matches!(self.interaction, Interaction::None) && (released || !down) {
			if let Interaction::Marquee { start, current, additive } = &self.interaction {
				let r = Bounds::from_corners(start.x, start.y, current.x, current.y);
				let mut picked: Vec<String> = self
					.doc
					.def
					.elements
					.iter()
					.filter(|e| {
						let id = &e.base().id;
						!self.doc.is_hidden(id) && !self.doc.is_locked(id) && geometry::bounds(e).intersects(&r)
					})
					.map(|e| e.base().id.clone())
					.collect();
				if *additive {
					let mut s = self.selection.clone();
					picked.retain(|id| !s.contains(id));
					s.extend(picked);
					self.selection = s;
				} else {
					self.selection = picked;
				}
			}
			self.interaction = Interaction::None;
			self.guides.clear();
		}

		// Cursor feedback.
		let icon = match &self.interaction {
			Interaction::Move { .. } => Some(CursorIcon::Grabbing),
			Interaction::Pan => Some(CursorIcon::Grabbing),
			Interaction::Resize { corner, .. } => Some(corner_cursor(*corner)),
			Interaction::Angle { .. } => Some(CursorIcon::Alias),
			_ if resp.hovered() => self.hover_cursor(view, pos, p, space),
			_ => None,
		};
		if let Some(icon) = icon {
			ui.ctx().set_cursor_icon(icon);
		}
	}

	fn hover_cursor(&self, view: Rect, pos: Pos2, p: Pos2, space: bool) -> Option<CursorIcon> {
		if space {
			return Some(CursorIcon::Grab);
		}
		if let Some((_, el)) = self.single_editable() {
			if geometry::angle_handles(&el)
				.iter()
				.any(|(_, x, y)| self.to_screen(view, pos2(*x, *y)).distance(pos) <= HANDLE_HIT)
			{
				return Some(CursorIcon::Alias);
			}
			let r = self.outline(view, geometry::bounds(&el));
			for (i, c) in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()].iter().enumerate() {
				if c.distance(pos) <= HANDLE_HIT {
					return Some(corner_cursor(i));
				}
			}
		}
		self.pick(p).map(|_| CursorIcon::Move)
	}

	fn start_interaction(&mut self, view: Rect, pos: Pos2, p: Pos2, mods: Modifiers, space: bool) -> Interaction {
		if space {
			return Interaction::Pan;
		}
		if let Some((id, el)) = self.single_editable() {
			for (handle, x, y) in geometry::angle_handles(&el) {
				if self.to_screen(view, pos2(x, y)).distance(pos) <= HANDLE_HIT {
					return Interaction::Angle { id, handle, moved: false };
				}
			}
			let b = geometry::bounds(&el);
			let r = self.outline(view, b);
			for (corner, c) in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()].iter().enumerate() {
				if c.distance(pos) <= HANDLE_HIT {
					return Interaction::Resize { id, corner, start: b, origin: Box::new(el), moved: false };
				}
			}
		}
		let hit = self.pick(p);
		let additive = mods.shift || mods.command;
		match hit {
			Some(hit) => {
				let members = self.doc.group_members(&hit);
				if additive {
					if self.selection.contains(&hit) {
						self.selection.retain(|id| !members.contains(id));
						return Interaction::None;
					}
					self.selection.retain(|id| !members.contains(id));
					self.selection.extend(members.iter().filter(|m| **m != hit).cloned());
					self.selection.push(hit);
				} else if !self.selection.contains(&hit) {
					let mut s: Vec<String> = members.into_iter().filter(|m| *m != hit).collect();
					s.push(hit);
					self.selection = s;
				}
				let origins = self
					.selection
					.iter()
					.filter(|id| !self.doc.is_locked(id))
					.filter_map(|id| self.doc.element(id).map(|e| (id.clone(), e.clone())))
					.collect();
				Interaction::Move { start: p, origins, moved: false }
			}
			None => {
				if !additive {
					self.selection.clear();
				}
				Interaction::Marquee { start: p, current: p, additive }
			}
		}
	}

	fn update_interaction(&mut self, view: Rect, p: Pos2, alt: bool, shift: bool) {
		let _ = view;
		match std::mem::replace(&mut self.interaction, Interaction::None) {
			Interaction::Move { start, origins, mut moved } => {
				let (mut dx, mut dy) = (p.x - start.x, p.y - start.y);
				if !moved && (dx * self.zoom).hypot(dy * self.zoom) < 2.0 {
					self.interaction = Interaction::Move { start, origins, moved };
					return;
				}
				if shift {
					// Constrain to the main axis.
					if dx.abs() > dy.abs() {
						dy = 0.0;
					} else {
						dx = 0.0;
					}
				}
				self.guides.clear();
				if let Some((_, first)) = origins.last() {
					// Grid snap moves the primary element onto the grid; the rest follow.
					let (x0, y0) = (first.base().x, first.base().y);
					dx = self.snap(x0 + dx, alt) - x0;
					dy = self.snap(y0 + dy, alt) - y0;
				}
				if self.smart_guides && !alt {
					let moving = origins.iter().map(|(_, e)| geometry::bounds(e)).reduce(|a, b| a.union(&b));
					if let Some(m) = moving {
						let ids: Vec<&String> = origins.iter().map(|(id, _)| id).collect();
						let (sx, gx) = self.snap_guides(&m, dx, true, &ids);
						let (sy, gy) = self.snap_guides(&m, dy, false, &ids);
						dx = sx;
						dy = sy;
						self.guides.extend(gx.map(|x| (true, x)));
						self.guides.extend(gy.map(|y| (false, y)));
					}
				}
				if !moved {
					let name = format!("Move {}", self.selection_label());
					self.doc.checkpoint(name);
					moved = true;
				}
				for (id, origin) in &origins {
					let mut el = origin.clone();
					geometry::translate(&mut el, dx, dy);
					if let Some(target) = self.doc.element_mut(id) {
						*target = el;
					}
				}
				self.doc.touch();
				self.interaction = Interaction::Move { start, origins, moved };
			}
			Interaction::Resize { id, corner, start, origin, mut moved } => {
				if !moved {
					self.doc.checkpoint(format!("Resize {}", origin.type_label()));
					moved = true;
				}
				let mut el = (*origin).clone();
				geometry::resize_from_corner(&mut el, start, corner, self.snap(p.x, alt), self.snap(p.y, alt));
				if let Some(target) = self.doc.element_mut(&id) {
					*target = el;
				}
				self.doc.touch();
				self.interaction = Interaction::Resize { id, corner, start, origin, moved };
			}
			Interaction::Angle { id, handle, mut moved } => {
				if !moved {
					let label = self.doc.element(&id).map(|e| e.type_label()).unwrap_or("");
					let what = match handle {
						AngleHandle::Start | AngleHandle::Rotation => "Rotate",
						AngleHandle::End => "Change sweep of",
					};
					self.doc.checkpoint(format!("{what} {label}"));
					moved = true;
				}
				if let Some(el) = self.doc.element_mut(&id) {
					geometry::apply_angle(el, handle, p.x, p.y, if shift { 15.0 } else { 1.0 });
				}
				self.doc.touch();
				self.interaction = Interaction::Angle { id, handle, moved };
			}
			Interaction::Marquee { start, additive, .. } => {
				self.interaction = Interaction::Marquee { start, current: p, additive };
			}
			Interaction::Pan => self.interaction = Interaction::Pan,
			Interaction::None => {}
		}
	}

	/// Snaps a move along one axis to the edges and centres of other
	/// elements and of the canvas. Returns the adjusted delta and the guide line.
	fn snap_guides(&self, moving: &Bounds, delta: f32, horizontal: bool, skip: &[&String]) -> (f32, Option<f32>) {
		let lines =
			|b: &Bounds| if horizontal { [b.left, b.center_x(), b.right] } else { [b.top, b.center_y(), b.bottom] };
		let canvas = Bounds::new(0.0, 0.0, self.doc.def.width as f32, self.doc.def.height as f32);
		let mut targets: Vec<f32> = lines(&canvas).to_vec();
		for e in &self.doc.def.elements {
			let id = &e.base().id;
			if !skip.contains(&id) && !self.doc.is_hidden(id) {
				targets.extend(lines(&geometry::bounds(e)));
			}
		}
		let threshold = GUIDE_SNAP / self.zoom;
		let mut best: Option<(f32, f32)> = None; // (distance, target)
		for m in lines(moving) {
			let moved = m + delta;
			for t in &targets {
				let d = (t - moved).abs();
				if d <= threshold && best.is_none_or(|(bd, _)| d < bd) {
					best = Some((d, *t - m));
				}
			}
		}
		match best {
			Some((_, snapped_delta)) => {
				let line = lines(moving)
					.iter()
					.map(|m| m + snapped_delta)
					.find(|v| targets.iter().any(|t| (t - v).abs() < 0.01));
				(snapped_delta, line)
			}
			None => (delta, None),
		}
	}

	fn draw_overlay(&self, painter: &egui::Painter, view: Rect) {
		let dash = |painter: &egui::Painter, r: Rect, color: Color32| {
			let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
			painter.extend(Shape::dashed_line(&pts, Stroke::new(1.5, color), 6.0, 4.0));
		};
		// Locked elements in the selection get a grey outline, others the selection colour.
		for (i, id) in self.selection.iter().enumerate() {
			let Some(el) = self.doc.element(id) else { continue };
			let primary = i + 1 == self.selection.len();
			let color = if self.doc.is_locked(id) {
				Color32::GRAY
			} else if primary {
				SELECT_COLOR
			} else {
				SECONDARY_COLOR
			};
			let r = self.outline(view, geometry::bounds(el));
			dash(painter, r, color);
		}
		if let Some((_, el)) = self.single_editable() {
			let r = self.outline(view, geometry::bounds(&el));
			for c in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()] {
				painter.rect_filled(Rect::from_center_size(c, vec2(7.0, 7.0)), 1.0, Color32::WHITE);
				painter.rect_stroke(
					Rect::from_center_size(c, vec2(7.0, 7.0)),
					1.0,
					Stroke::new(1.0, SELECT_COLOR),
					egui::StrokeKind::Outside,
				);
			}
			if let Some((cx, cy)) = geometry::angle_center(&el) {
				let center = self.to_screen(view, pos2(cx, cy));
				for (handle, x, y) in geometry::angle_handles(&el) {
					let hp = self.to_screen(view, pos2(x, y));
					painter.line_segment(
						[center, hp],
						Stroke::new(1.0, Color32::from_rgba_unmultiplied(0x4D, 0xC3, 0xFF, 120)),
					);
					let fill = if handle == AngleHandle::End {
						Color32::from_rgb(0xFF, 0x8C, 0x00)
					} else {
						Color32::from_rgb(0x4D, 0xC3, 0xFF)
					};
					painter.circle(hp, 5.5, fill, Stroke::new(1.5, Color32::WHITE));
				}
			}
		}
		for (vertical, v) in &self.guides {
			let (a, b) = if *vertical {
				(self.to_screen(view, pos2(*v, -10000.0)), self.to_screen(view, pos2(*v, 10000.0)))
			} else {
				(self.to_screen(view, pos2(-10000.0, *v)), self.to_screen(view, pos2(10000.0, *v)))
			};
			painter.line_segment([a, b], Stroke::new(1.0, GUIDE_COLOR));
		}
		if let Interaction::Marquee { start, current, .. } = &self.interaction {
			let r = Rect::from_two_pos(self.to_screen(view, *start), self.to_screen(view, *current));
			painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(0x4D, 0xC3, 0xFF, 30));
			painter.rect_stroke(r, 0.0, Stroke::new(1.0, SELECT_COLOR), egui::StrokeKind::Inside);
		}
	}

	fn draw_rulers(&self, painter: &egui::Painter, full: Rect, view: Rect) {
		let bg = Color32::from_gray(0x24);
		let fg = Color32::from_gray(0x9A);
		let top = Rect::from_min_max(pos2(view.left(), full.top()), pos2(view.right(), view.top()));
		let left = Rect::from_min_max(pos2(full.left(), view.top()), pos2(view.left(), view.bottom()));
		painter.rect_filled(top, 0.0, bg);
		painter.rect_filled(left, 0.0, bg);
		painter.rect_filled(Rect::from_min_max(full.min, view.min), 0.0, bg);
		// Label spacing: the smallest of 10/20/50/100/200 px that's ≥ 50 screen px apart.
		let step =
			[10.0, 20.0, 50.0, 100.0, 200.0, 500.0].into_iter().find(|s| s * self.zoom >= 50.0).unwrap_or(1000.0);
		let minor = step / 5.0;
		let font = FontId::monospace(9.0);
		let c0 = self.to_canvas(view, view.min);
		let c1 = self.to_canvas(view, view.max);
		let mut v = (c0.x / minor).floor() * minor;
		while v <= c1.x {
			let sx = self.to_screen(view, pos2(v, 0.0)).x;
			let major = (v / step).round() * step == v;
			let len = if major { RULER } else { RULER * 0.3 };
			painter.line_segment([pos2(sx, top.bottom() - len), pos2(sx, top.bottom())], Stroke::new(1.0, fg));
			if major {
				painter.text(pos2(sx + 2.0, top.top() + 1.0), Align2::LEFT_TOP, format!("{v:.0}"), font.clone(), fg);
			}
			v += minor;
		}
		let mut v = (c0.y / minor).floor() * minor;
		while v <= c1.y {
			let sy = self.to_screen(view, pos2(0.0, v)).y;
			let major = (v / step).round() * step == v;
			let len = if major { RULER } else { RULER * 0.3 };
			painter.line_segment([pos2(left.right() - len, sy), pos2(left.right(), sy)], Stroke::new(1.0, fg));
			if major {
				painter.text(pos2(left.left() + 1.0, sy + 2.0), Align2::LEFT_TOP, format!("{v:.0}"), font.clone(), fg);
			}
			v += minor;
		}
		// Selection extent and cursor on the rulers.
		if let Some(b) = self.selected_bounds().into_iter().map(|(_, b)| b).reduce(|a, b| a.union(&b)) {
			let a = self.to_screen(view, pos2(b.left, b.top));
			let z = self.to_screen(view, pos2(b.right, b.bottom));
			let hl = Color32::from_rgba_unmultiplied(0x4D, 0xC3, 0xFF, 60);
			painter.rect_filled(
				Rect::from_min_max(pos2(a.x, top.top()), pos2(z.x, top.bottom())).intersect(top),
				0.0,
				hl,
			);
			painter.rect_filled(
				Rect::from_min_max(pos2(left.left(), a.y), pos2(left.right(), z.y)).intersect(left),
				0.0,
				hl,
			);
		}
		if let Some(c) = self.cursor {
			let s = self.to_screen(view, c);
			painter.line_segment([pos2(s.x, top.top()), pos2(s.x, top.bottom())], Stroke::new(1.0, SELECT_COLOR));
			painter.line_segment([pos2(left.left(), s.y), pos2(left.right(), s.y)], Stroke::new(1.0, SELECT_COLOR));
		}
	}
}

/// The element's name, or something recognisable when it has none.
fn list_name(e: &Element) -> String {
	let b = e.base();
	if !b.name.is_empty() {
		return b.name.clone();
	}
	let hint = match e {
		Element::Text(t) => format!("\"{}\"", t.text),
		Element::LabelValue(lv) => format!("\"{}\"", lv.label),
		Element::Warning(w) => format!("\"{}\"", w.label),
		_ => b.source().map(str::to_string).unwrap_or_default(),
	};
	format!("({hint})").replace("()", "")
}

fn corner_cursor(corner: usize) -> CursorIcon {
	match corner {
		0 => CursorIcon::ResizeNorthWest,
		1 => CursorIcon::ResizeNorthEast,
		2 => CursorIcon::ResizeSouthWest,
		_ => CursorIcon::ResizeSouthEast,
	}
}

fn bar(ui: &mut Ui, fraction: f32) {
	let (rect, _) = ui.allocate_exact_size(vec2(80.0, 10.0), Sense::hover());
	ui.painter().rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
	let fill = Rect::from_min_size(rect.min, vec2(rect.width() * fraction.clamp(0.0, 1.0), rect.height()));
	ui.painter().rect_filled(fill, 2.0, Color32::from_rgb(0xFF, 0x8C, 0x00));
}

/// Makes relative image paths absolute, for pasting or inserting elsewhere.
fn absolutize_images(elements: &mut [Element], dir: &Path) {
	let fix = |p: &mut String| {
		if !p.is_empty() && Path::new(p.as_str()).is_relative() {
			*p = dir.join(&*p).display().to_string();
		}
	};
	for e in elements {
		match e {
			Element::Needle(n) => {
				if let Some(p) = n.image_path.as_mut() {
					fix(p);
				}
			}
			Element::Image(i) => fix(&mut i.image_path),
			Element::Gif(g) => fix(&mut g.image_path),
			_ => {}
		}
	}
}

#[derive(Clone, Copy, PartialEq)]
enum Align {
	Left,
	CenterH,
	Right,
	Top,
	CenterV,
	Bottom,
	CanvasH,
	CanvasV,
	Canvas,
}

impl Align {
	const ALL: [Align; 9] = [
		Align::Left,
		Align::CenterH,
		Align::Right,
		Align::Top,
		Align::CenterV,
		Align::Bottom,
		Align::CanvasH,
		Align::CanvasV,
		Align::Canvas,
	];

	fn name(self) -> &'static str {
		match self {
			Align::Left => "Align left",
			Align::CenterH => "Align centres horizontally",
			Align::Right => "Align right",
			Align::Top => "Align top",
			Align::CenterV => "Align centres vertically",
			Align::Bottom => "Align bottom",
			Align::CanvasH => "Centre horizontally on canvas",
			Align::CanvasV => "Centre vertically on canvas",
			Align::Canvas => "Centre on canvas",
		}
	}

	fn to_canvas(self) -> bool {
		matches!(self, Align::CanvasH | Align::CanvasV | Align::Canvas)
	}
}

impl eframe::App for DesignerApp {
	fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
		let ctx = ui.ctx().clone();
		if ctx.input(|i| i.viewport().close_requested()) && self.doc.is_dirty() && !self.allow_close {
			ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
			self.pending = Some(Pending::Quit);
		}
		let title = format!("{} — Gauge Designer", self.doc.title());
		if title != self.title {
			ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
			self.title = title;
		}

		self.shortcuts(&ctx);
		self.poll_jobs(&ctx);
		self.refresh_analysis();
		if self.auto_cost
			&& self.cost_revision != self.doc.revision
			&& !matches!(
				self.interaction,
				Interaction::Move { .. } | Interaction::Resize { .. } | Interaction::Angle { .. }
			) {
			self.measure_cost();
		}

		egui::Panel::top("menu").show(ui, |ui| {
			self.menu_bar(ui);
			self.toolbox(ui);
		});
		egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
		egui::Panel::left("elements").resizable(true).default_size(250.0).show(ui, |ui| self.element_list(ui));
		egui::Panel::right("inspector").resizable(true).default_size(360.0).show(ui, |ui| {
			egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
				self.properties(ui);
				ui.add_space(8.0);
				egui::CollapsingHeader::new(RichText::new("Test values").strong())
					.default_open(true)
					.show(ui, |ui| self.test_values(ui));
				let (errors, warnings) = (
					self.issues.iter().filter(|i| i.severity == Severity::Error).count(),
					self.issues.iter().filter(|i| i.severity == Severity::Warning).count(),
				);
				let title = match (errors, warnings) {
					(0, 0) => "Validation".to_string(),
					(e, w) => format!("Validation ({e} errors, {w} warnings)"),
				};
				egui::CollapsingHeader::new(RichText::new(title).strong())
					.id_salt("validation")
					.default_open(errors > 0)
					.show(ui, |ui| self.validation(ui));
				egui::CollapsingHeader::new(RichText::new("Frame cost").strong())
					.default_open(false)
					.show(ui, |ui| self.cost_panel(ui));
			});
		});
		self.ensure_rendered(&ctx);
		egui::CentralPanel::no_frame().show(ui, |ui| self.canvas_view(ui));

		self.deploy_window(&ctx);
		self.history_window(&ctx);
		if let Some(config) = &mut self.config {
			let mut open = true;
			let request = config.window(&ctx, &mut open);
			if !open {
				self.config = None;
			}
			if let Some(path) = request {
				self.guard(Pending::Open(Some(path)));
			}
		}
		self.pending_modal(&ctx);
	}
}
