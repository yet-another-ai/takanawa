# Godot GDExtension

The Godot GDExtension package exposes Takanawa downloads to GDScript through a
`TakanawaDownload` node. It is intended for Godot projects that do not use the
C# runtime. Godot C# projects can continue to use the `YetAnotherAI.Takanawa`
NuGet package.

## Install

Download `takanawa-gdextension.zip` from a GitHub release and copy its
`addons/takanawa` directory into your Godot project.

The addon includes desktop libraries for Linux, macOS, and Windows, Android
libraries for Godot export ABIs, and an iOS `TakanawaGDExtension.xcframework`.
iOS support depends on Godot and godot-rust mobile GDExtension support, which is
less mature than the desktop path.

## GDScript Usage

Add a `TakanawaDownload` node to a scene and configure it with a `Dictionary`.
Both snake_case and camelCase option names are accepted.

```gdscript
extends Node

@onready var download: TakanawaDownload = $TakanawaDownload

func _ready() -> void:
    download.progress.connect(_on_progress)
    download.speed.connect(_on_speed)
    download.completed.connect(_on_completed)
    download.failed.connect(_on_failed)

    if not download.configure({
        "url": "https://example.com/file.bin",
        "target_path": "user://file.bin",
        "parallelism": 4,
        "max_parallel_chunks": 8,
        "hash": {
            "kind": "sha256",
            "expected": "00".repeat(32),
        },
    }):
        push_error(download.last_error())
        return

    download.start()

func _on_progress(snapshot: Dictionary) -> void:
    print("%s/%s" % [snapshot["downloaded_bytes"], snapshot["content_len"]])

func _on_speed(snapshot: Dictionary) -> void:
    print("%s B/s" % snapshot["bytes_per_second"])

func _on_completed(snapshot: Dictionary) -> void:
    print("download complete: ", snapshot)

func _on_failed(message: String, snapshot: Dictionary) -> void:
    push_error("%s: %s" % [message, snapshot])
```

## API

`TakanawaDownload` methods:

- `configure(options: Dictionary) -> bool`
- `start() -> bool`
- `pause() -> bool`
- `cancel() -> bool`
- `snapshot() -> Dictionary`
- `speed_snapshot() -> Dictionary`
- `bitmap() -> PackedByteArray`
- `close() -> void`
- `last_error() -> String`

Signals:

- `progress(snapshot: Dictionary)`
- `speed(snapshot: Dictionary)`
- `completed(snapshot: Dictionary)`
- `failed(message: String, snapshot: Dictionary)`
- `cancelled(snapshot: Dictionary)`

Snapshot byte counters are decimal strings, matching the JavaScript and Tauri
bindings. This avoids precision surprises when values cross language boundaries.

## Build Locally

```sh
cargo test -p takanawa-gdextension --locked
mise run package:gdextension-desktop
mise run dist:gdextension
```

Android and Apple builds use separate tasks:

```sh
mise run package:gdextension-android
mise run package:gdextension-ios
```
