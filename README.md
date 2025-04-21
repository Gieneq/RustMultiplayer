# 🎮 Rust Multiplayer Minigame

A mini multiplayer game written entirely in Rust — featuring an async Tokio TCP server, WGPU-based GUI client, and a CLI client alternative. Designed to be real-time, modular, and highly testable.

Features:
- ✔️ Lobby system
- ✔️ Hider/Seeker role logic
- ✔️ Custom JSON protocol
- ✔️ Procedurally generated map
- ✔️ Full async & multithreaded code
- ✔️ Unit & integration tested
- ✔️ Ingame Chat (CLI only so far)

## Quick Try

You can download the latest release [here](https://github.com/Gieneq/RustMultiplayer/releases/tag/v0.1.0)

Run server
```cmd
rust_multiplayer server
```

Start client (new player):
 ```cmd
 rust_multiplayer player
 ```

## Lobby

Players who connect are placed in the lobby.
Once all click Ready, the game begins:
<p align="center"> <img src="res/hide_n_seek_opening.gif"> </p>

## Gameplay

One player is randomly selected as the Seeker.
The Seeker left-clicks entities to uncover hidden players:
- ❌ A wrong guess costs a heart
- 💡 Finding a real hider uncovers them
- 🕒 Game ends when time or lives run out

<p align="center"> <img src="res/hide_n_seek_gameplay.gif"> </p>

## Architecture

Server:
- Async TCP server built with [Tokio]
- 2 main tasks loops:
  - networking handling incomming connections, handling requests, forming responses
  - game world managing game states, updating entities
- Players are decoupled from entities (ECS-style)

Client:
- Abstractian over TCP request-responses
- Multithreading friendly
- WGPU frontend

## Tests & Running

Tests:
- Unit tests coves complex logic
- Integration tests covers cleint-server requersts and state transition

Run:
- server mode `rust_multiplayer.exe server`, server will be exposed on default address,
- client mode `rust_multiplayer.exe player`, client will be conencted to server, server assigns random player name
- cli mode `rust_multiplayer.exe client`, in development, executing requests

## Crates / Tools Used
[Tokio] – async runtime
[Serde] – serialization
[WGPU] – GPU rendering
[Clap] – command-line parsing
[thiserror] – error handling

