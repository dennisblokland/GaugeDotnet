# muOS artwork

Box art, preview and description for the two launchers in muOS's Ports list
(`External - Ports` catalogue). The deploy scripts upload them next to the
launchers; muOS matches them to `GaugeDotnet.sh` / `GaugeRust.sh` by name.

Both launchers use the same image, a real render from the Rust renderer:

```bash
cd rust
cargo run --release -- builtin turboxs -o /tmp/t.png --value 85 --peak 110 --max 120 --title BOOST --unit KPA
```

scaled to 320×240 (half the screen size) and saved as both `GaugeDotnet.png`
and `GaugeRust.png`.
