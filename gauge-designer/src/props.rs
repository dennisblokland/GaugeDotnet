//! Property panel widgets, driven by the table in [`crate::fields`].

use eframe::egui::{self, Color32, ComboBox, DragValue, RichText, TextEdit, Ui};
use gauge_render::color::Rgba;
use gauge_render::text::FONT_KEYS;
use serde_json::{Map, Value};

use crate::fields::{Field, Kind, IMAGE_MODES};
use crate::validate::{check_date_format, check_number_format};

/// What the widgets need besides the value.
pub struct PropContext<'a> {
	/// Channel names offered for data sources (ECU channels, then calculated ones).
	pub sources: &'a [String],
	/// Folder file pickers start in.
	pub start_dir: Option<&'a std::path::Path>,
}

pub const WARN: Color32 = Color32::from_rgb(0xFF, 0xB0, 0x30);
pub const ERROR: Color32 = Color32::from_rgb(0xFF, 0x60, 0x60);

/// Draws the rows of `fields` for the JSON object `obj`. Returns the label
/// of the property that changed, if any.
pub fn edit_fields(
	ui: &mut Ui,
	grid_id: &str,
	fields: &[Field],
	obj: &mut Map<String, Value>,
	ctx: &PropContext,
) -> Option<Field> {
	let mut changed = None;
	// Rows between section headings share a grid so labels line up.
	let mut groups: Vec<(Option<&Field>, Vec<&Field>)> = vec![(None, Vec::new())];
	for field in fields {
		if field.kind == Kind::Section {
			groups.push((Some(field), Vec::new()));
		} else {
			groups.last_mut().unwrap().1.push(field);
		}
	}
	for (i, (heading, rows)) in groups.into_iter().enumerate() {
		if let Some(h) = heading {
			ui.add_space(4.0);
			ui.separator();
			ui.label(RichText::new(h.label).strong());
		}
		egui::Grid::new((grid_id, i)).num_columns(2).spacing([8.0, 4.0]).striped(false).show(ui, |ui| {
			for field in rows {
				ui.label(field.label);
				ui.push_id(field.key, |ui| {
					if field_widget(ui, field, obj, ctx) {
						changed = Some(*field);
					}
				});
				ui.end_row();
			}
		});
	}
	changed
}

fn str_of(obj: &Map<String, Value>, key: &str) -> String {
	obj.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

fn num_of(obj: &Map<String, Value>, key: &str) -> f64 {
	obj.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn field_widget(ui: &mut Ui, field: &Field, obj: &mut Map<String, Value>, ctx: &PropContext) -> bool {
	let key = field.key;
	match field.kind {
		Kind::Section => false,
		Kind::Text => {
			let mut s = str_of(obj, key);
			let changed = ui.add(TextEdit::singleline(&mut s).desired_width(f32::INFINITY)).changed();
			if changed {
				obj.insert(key.into(), Value::String(s));
			}
			changed
		}
		Kind::Float(min, max, step) => {
			let mut v = num_of(obj, key);
			let changed = ui
				.add(DragValue::new(&mut v).range(min as f64..=max as f64).speed(step as f64).max_decimals(2))
				.changed();
			if changed {
				obj.insert(key.into(), Value::from(v));
			}
			changed
		}
		Kind::Int(min, max) => {
			let mut v = num_of(obj, key) as i64;
			let changed = ui.add(DragValue::new(&mut v).range(min as i64..=max as i64)).changed();
			if changed {
				obj.insert(key.into(), Value::from(v));
			}
			changed
		}
		Kind::Byte => {
			let mut v = num_of(obj, key) as u8;
			let changed = ui.add(egui::Slider::new(&mut v, 0..=255)).changed();
			if changed {
				obj.insert(key.into(), Value::from(v));
			}
			changed
		}
		Kind::Bool => {
			let mut v = obj.get(key).and_then(Value::as_bool).unwrap_or(false);
			let changed = ui.checkbox(&mut v, "").changed();
			if changed {
				obj.insert(key.into(), Value::Bool(v));
			}
			changed
		}
		Kind::Color => color_widget(ui, key, obj),
		Kind::Source => {
			let current = str_of(obj, key);
			let mut picked = current.clone();
			let known = current.is_empty() || ctx.sources.iter().any(|s| s.eq_ignore_ascii_case(&current));
			let shown = if current.is_empty() {
				"(none)".to_string()
			} else if known {
				current.clone()
			} else {
				format!("⚠ {current}")
			};
			ComboBox::from_id_salt(key).selected_text(shown).width(ui.available_width()).height(320.0).show_ui(
				ui,
				|ui| {
					ui.selectable_value(&mut picked, String::new(), "(none)");
					if !known {
						ui.selectable_value(&mut picked, current.clone(), format!("⚠ {current} (unknown)"));
					}
					for s in ctx.sources {
						ui.selectable_value(&mut picked, s.clone(), s);
					}
				},
			);
			let changed = picked != current;
			if changed {
				obj.insert(key.into(), if picked.is_empty() { Value::Null } else { Value::String(picked) });
			}
			changed
		}
		Kind::Font => {
			let current = str_of(obj, key);
			let mut picked = current.clone();
			ComboBox::from_id_salt(key).selected_text(&current).width(ui.available_width()).show_ui(ui, |ui| {
				for f in FONT_KEYS {
					ui.selectable_value(&mut picked, f.to_string(), f);
				}
			});
			let changed = picked != current;
			if changed {
				obj.insert(key.into(), Value::String(picked));
			}
			changed
		}
		Kind::ImageMode => {
			let current = num_of(obj, key) as usize;
			let mut picked = current;
			ComboBox::from_id_salt(key).selected_text(IMAGE_MODES.get(current).copied().unwrap_or("?")).show_ui(
				ui,
				|ui| {
					for (i, name) in IMAGE_MODES.iter().enumerate() {
						ui.selectable_value(&mut picked, i, *name);
					}
				},
			);
			let changed = picked != current;
			if changed {
				obj.insert(key.into(), Value::from(picked));
			}
			changed
		}
		Kind::Image(optional) => {
			let mut s = str_of(obj, key);
			let mut changed = false;
			ui.horizontal(|ui| {
				if ui.button("…").on_hover_text("Choose a file").clicked() {
					let mut dialog =
						rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif", "bmp"]);
					if let Some(dir) = ctx.start_dir {
						dialog = dialog.set_directory(dir);
					}
					if let Some(path) = dialog.pick_file() {
						s = path.display().to_string();
						changed = true;
					}
				}
				changed |= ui.add(TextEdit::singleline(&mut s).desired_width(f32::INFINITY)).changed();
			});
			if changed {
				obj.insert(key.into(), if s.is_empty() && optional { Value::Null } else { Value::String(s) });
			}
			changed
		}
		Kind::Limit(lower) => {
			let off = if lower { f32::MIN } else { f32::MAX } as f64;
			let v = num_of(obj, key);
			let mut on = (v - off).abs() > f32::MAX as f64 * 1e-6;
			let mut value = if on {
				v
			} else if lower {
				0.0
			} else {
				100.0
			};
			let mut changed = false;
			ui.horizontal(|ui| {
				changed |= ui
					.checkbox(&mut on, "")
					.on_hover_text(if lower { "Off: no lower limit" } else { "Off: no upper limit" })
					.changed();
				ui.add_enabled_ui(on, |ui| {
					changed |= ui.add(DragValue::new(&mut value).speed(1.0).max_decimals(2)).changed()
				});
			});
			if changed {
				obj.insert(key.into(), Value::from(if on { value } else { off }));
			}
			changed
		}
		Kind::NumberFormat | Kind::DateFormat => {
			let mut s = str_of(obj, key);
			let mut changed = false;
			ui.vertical(|ui| {
				changed = ui.add(TextEdit::singleline(&mut s).desired_width(f32::INFINITY)).changed();
				let problem =
					if field.kind == Kind::NumberFormat { check_number_format(&s) } else { check_date_format(&s) };
				if let Some(p) = problem {
					ui.label(RichText::new(format!("⚠ {p}")).small().color(WARN));
				}
			});
			if changed {
				obj.insert(key.into(), Value::String(s));
			}
			changed
		}
	}
}

/// Hex text plus a colour picker. Any text is stored (so typing works);
/// text that isn't a colour is outlined red (it renders black on the device).
fn color_widget(ui: &mut Ui, key: &str, obj: &mut Map<String, Value>) -> bool {
	let mut s = str_of(obj, key);
	let parsed = Rgba::parse(&s);
	let mut changed = false;
	ui.horizontal(|ui| {
		let mut c = parsed.map(|c| Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)).unwrap_or(Color32::BLACK);
		if ui.color_edit_button_srgba(&mut c).changed() {
			let [r, g, b, a] = c.to_srgba_unmultiplied();
			s = if a == 255 { format!("#{r:02X}{g:02X}{b:02X}") } else { format!("#{a:02X}{r:02X}{g:02X}{b:02X}") };
			changed = true;
		}
		let mut edit = TextEdit::singleline(&mut s).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace);
		if parsed.is_none() {
			edit = edit.text_color(ERROR);
		}
		changed |= ui.add(edit).on_hover_text("#RRGGBB or #AARRGGBB").changed();
	});
	if changed {
		obj.insert(key.into(), Value::String(s));
	}
	changed
}
