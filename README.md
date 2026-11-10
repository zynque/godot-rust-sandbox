# godot-rust-sandbox
Toy Project for Learning Rust with Godot

<img width="760" height="498" alt="Screenshot 2026-11-10 102520" src="https://github.com/user-attachments/assets/fcdff55d-d29b-40eb-94d9-9e74aa1a9b50" />

See the Godot-Rust documentation here: https://godot-rust.github.io/

The following need to be installed in order for this to run:
 * Godot
 * Rust
* Visual Studio Community Edition (For C++ linker)

As for the editor, I prefer Visual Studio Code with the rust-analyzer plugin

__Note__ that the Godot version and the godot-rust version need to be compatible. Godot-Rust docs were out of date in this regard. Look at release notes for godot-rust to see exactly what is supported when. In this project the godot-rust version is currently set to "0.5.5" and Godot is 4.5.

Once the required tools are installed, run `build.bat` from the repository root, or run `cargo build` from the `rust` folder (the demo scene, which also builds the `addons/growth` parcel library) and from the `addons/beatsmr/rust` folder (the parcel renderer), and then import the project in Godot and run it.
