# Oxide

## Installation

### Quick Install (Recommended)

```bash
curl -sSL https://raw.githubusercontent.com/Maksym-Zhuk/oxide/main/install.sh | bash
```

After installation, add to your PATH (if not already):

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

Or for zsh:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Manual Installation

Download the latest release for your platform:

- [Linux x86_64](https://github.com/Maksym-Zhuk/oxide/releases/latest/download/oxide-linux-x86_64.tar.gz)
- [macOS Intel](https://github.com/Maksym-Zhuk/oxide/releases/latest/download/oxide-macos-x86_64.tar.gz)
- [macOS Apple Silicon](https://github.com/Maksym-Zhuk/oxide/releases/latest/download/oxide-macos-aarch64.tar.gz)
- [Windows](https://github.com/Maksym-Zhuk/oxide/releases/latest/download/oxide-windows-x86_64.zip)

## Usage

```bash
oxide create my-app
```
