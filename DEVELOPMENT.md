# Cortex — Development Setup

## Quick start (all platforms)

```sh
npm install
npm run tauri:dev
```

That's it on macOS and Linux. On Windows, see the Windows section below.

---

## macOS

**Prerequisites:**
- Xcode Command Line Tools: `xcode-select --install`
- Rust via rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Node.js 18+ via [nvm](https://github.com/nvm-sh/nvm) or [Homebrew](https://brew.sh): `brew install node`

**Targets:**
- Apple Silicon: `aarch64-apple-darwin` (default on M1/M2/M3 macs)
- Intel: `x86_64-apple-darwin`
- Universal binary: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`

**Run dev:**
```sh
npm install
npm run tauri:dev
```

**Production build:**
```sh
npm run tauri:build
```

---

## Linux (Ubuntu / Debian)

**Prerequisites:**
```sh
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

Then install Rust and Node:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Node via nvm or apt
```

**Run dev:**
```sh
npm install
npm run tauri:dev
```

---

## Windows

**Prerequisites:**
1. [Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) with the **Desktop development with C++** workload
2. Rust via rustup (MSVC toolchain):
   ```
   rustup toolchain install stable-x86_64-pc-windows-msvc
   rustup default stable-x86_64-pc-windows-msvc
   ```
3. Node.js 18+

**MinGW conflict fix** (if you have MSYS2/MinGW on PATH):

MinGW ships its own `link.exe` which conflicts with the MSVC linker. Create a local cargo config to explicitly point at the MSVC linker:

```toml
# .cargo/config.toml  (this file is .gitignored — create it locally)
[target.x86_64-pc-windows-msvc]
linker = "C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Tools\\MSVC\\14.44.35207\\bin\\Hostx64\\x64\\link.exe"
```

Adjust the MSVC version path to match your installation (`dir "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\"` to find it).

**Run dev (recommended):**
```bat
dev.bat
```
`dev.bat` calls `vcvars64.bat` automatically before launching Tauri, ensuring the MSVC linker is on PATH without needing `.cargo/config.toml`.

**Alternative (if PATH is clean):**
```sh
npm run tauri:dev
```

**Production build:**
```bat
build.bat
```

---

## Database locations

| Platform | Path |
|----------|------|
| Windows | `%APPDATA%\com.cortex.app\` |
| macOS | `~/Library/Application Support/com.cortex.app/` |
| Linux | `~/.local/share/com.cortex.app/` |

The SQLite database is `cortex.db` and the vault mirror is in `vault/` inside that directory.

The embedding model files (downloaded on first launch) are stored in `models/` inside the same directory (~70MB total).

---

## MCP server

The MCP server binds to `127.0.0.1:7340` on startup. Connect any Claude Code / Cursor agent with:

```json
{
  "mcpServers": {
    "cortex": {
      "command": "nc",
      "args": ["127.0.0.1", "7340"]
    }
  }
}
```

See `MCP_USAGE.md` for full usage details.
