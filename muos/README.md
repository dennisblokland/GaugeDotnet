# muOS artwork

Box art, preview and description for the launcher in muOS's Ports list
(`External - Ports` catalogue). The deploy script uploads them next to the
launcher; muOS matches them to `GaugeRust.sh` by name. On
[ROCKNIX](../rocknix/README.md) the same image and description go into `gamelist.xml`.

The image is a real render from the renderer:

```bash
cd rust
cargo run --release -- builtin turboxs -o /tmp/t.png --value 85 --peak 110 --max 120 --title BOOST --unit KPA
```

scaled to 320×240 (half the screen size) and saved as `GaugeRust.png`.
