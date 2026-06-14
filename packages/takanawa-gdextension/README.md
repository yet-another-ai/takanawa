# Takanawa GDExtension

Godot 4 GDExtension bindings for Takanawa range downloads.

Copy the `addons/takanawa` directory from the release archive into your Godot
project, then add a `TakanawaDownload` node to a scene.

```gdscript
extends Node

@onready var download: TakanawaDownload = $TakanawaDownload

func _ready() -> void:
    download.progress.connect(func(snapshot: Dictionary) -> void:
        print("downloaded: ", snapshot["downloaded_bytes"])
    )
    download.completed.connect(func(snapshot: Dictionary) -> void:
        print("completed: ", snapshot)
    )
    download.failed.connect(func(message: String, snapshot: Dictionary) -> void:
        push_error("%s: %s" % [message, snapshot])
    )

    var ok := download.configure({
        "url": "https://example.com/file.bin",
        "target_path": "user://file.bin",
        "parallelism": 4,
        "max_parallel_chunks": 8,
    })
    if ok:
        download.start()
```

The API accepts both snake_case and camelCase option names to match GDScript and
the existing JavaScript/Tauri bindings.
