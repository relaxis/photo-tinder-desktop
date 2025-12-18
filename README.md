# Photo Tinder

A fast, keyboard-driven desktop app for triaging and ranking large photo collections. Swipe through thousands of photos like Tinder, then rank your favorites with side-by-side comparisons.

![Photo Tinder Screenshot](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![License](https://img.shields.io/badge/License-MIT-green)

## Download

**[View All Releases](https://github.com/relaxis/photo-tinder-desktop/releases)**

### Latest Version (v1.0.1)

| Platform | Download | Size |
|----------|----------|------|
| **Windows** | [Photo.Tinder_1.0.1_x64-setup.exe](https://github.com/relaxis/photo-tinder-desktop/releases/download/v1.0.1/Photo.Tinder_1.0.1_x64-setup.exe) | ~3 MB |
| **Windows (MSI)** | [Photo.Tinder_1.0.1_x64_en-US.msi](https://github.com/relaxis/photo-tinder-desktop/releases/download/v1.0.1/Photo.Tinder_1.0.1_x64_en-US.msi) | ~3 MB |
| **macOS (Apple Silicon)** | [Photo.Tinder_1.0.1_aarch64.dmg](https://github.com/relaxis/photo-tinder-desktop/releases/download/v1.0.1/Photo.Tinder_1.0.1_aarch64.dmg) | ~2 MB |
| **macOS (Intel)** | [Photo.Tinder_1.0.1_x64.dmg](https://github.com/relaxis/photo-tinder-desktop/releases/download/v1.0.1/Photo.Tinder_1.0.1_x64.dmg) | ~2 MB |
| **Linux (AppImage)** | [Photo.Tinder_1.0.1_amd64.AppImage](https://github.com/relaxis/photo-tinder-desktop/releases/download/v1.0.1/Photo.Tinder_1.0.1_amd64.AppImage) | ~73 MB |
| **Linux (Debian/Ubuntu)** | [Photo.Tinder_1.0.1_amd64.deb](https://github.com/relaxis/photo-tinder-desktop/releases/download/v1.0.1/Photo.Tinder_1.0.1_amd64.deb) | ~2.5 MB |
| **Linux (Fedora/RHEL)** | [Photo.Tinder-1.0.1-1.x86_64.rpm](https://github.com/relaxis/photo-tinder-desktop/releases/download/v1.0.1/Photo.Tinder-1.0.1-1.x86_64.rpm) | ~2.5 MB |

## Features

### Triage Mode
Quickly sort through photos with swipe gestures or keyboard shortcuts:
- **Right / Accept** - Move photo to your "Accepted" folder
- **Left / Reject** - Move photo to your "Rejected" folder
- **Down / Skip** - Skip for now, decide later
- **Up / Undo** - Undo your last decision

### Ranking Mode
Compare photos side-by-side to find your absolute best shots:
- Uses TrueSkill rating algorithm for accurate rankings
- Smart matchmaking prioritizes uncertain comparisons
- View your Top 50 photos on the leaderboard

### Photo Browser
Browse and manage your sorted photos:
- View accepted or rejected photos
- Sort by ranking, date, or filename
- Click to view full-size with pinch-to-zoom

### Supported Formats
- **Common**: JPG, JPEG, PNG, WebP, GIF, BMP, TIFF
- **Modern**: HEIC, HEIF, AVIF, JXL
- **RAW**: CR2, CR3, NEF, ARW, DNG, ORF, RW2, RAF, and 20+ more

## Quick Start

1. **Download** the appropriate version for your platform
2. **Run** the installer or AppImage
3. **Add source folders** containing photos to triage
4. **Set destination folders** for accepted and rejected photos
5. **Start swiping!**

## Keyboard Shortcuts

### Triage Mode
| Key | Action |
|-----|--------|
| `→` | Accept photo |
| `←` | Reject photo |
| `↓` or `S` | Skip photo |
| `↑` or `U` | Undo last action |

### Ranking Mode
| Key | Action |
|-----|--------|
| `A` | Left photo wins |
| `D` | Right photo wins |
| `S` | Tie |
| `W` | Skip comparison |
| `U` | Undo |

## Installation

### Windows
1. Download the `.exe` installer
2. Run it and follow the prompts
3. If SmartScreen appears, click "More info" → "Run anyway"

### macOS

1. Download the `.dmg` for your Mac (Apple Silicon or Intel)
2. Open the `.dmg` and drag Photo Tinder to Applications
3. **First launch**: Right-click the app → "Open" → click "Open" in the dialog
   (Required because the app isn't notarized with Apple)

### Linux (Recommended: .deb or .rpm)

**Debian/Ubuntu** - easiest option, just run:
```bash
sudo dpkg -i ~/Downloads/Photo.Tinder_1.0.1_amd64.deb && sudo apt-get install -f
```

**Fedora/RHEL:**
```bash
sudo rpm -i ~/Downloads/Photo.Tinder-1.0.1-1.x86_64.rpm
```

### Linux (AppImage - Universal)

AppImages need to be made executable before running (browsers strip this for security).

**One-liner to download and run:**
```bash
curl -L https://github.com/relaxis/photo-tinder-desktop/releases/download/v1.0.1/Photo.Tinder_1.0.1_amd64.AppImage -o ~/Photo-Tinder.AppImage && chmod +x ~/Photo-Tinder.AppImage && ~/Photo-Tinder.AppImage
```

**Or if already downloaded:**
```bash
chmod +x ~/Downloads/Photo.Tinder_1.0.1_amd64.AppImage && ~/Downloads/Photo.Tinder_1.0.1_amd64.AppImage
```

## Building from Source

Requires: Node.js 18+, Rust 1.70+

```bash
# Clone the repository
git clone https://github.com/relaxis/photo-tinder-desktop.git
cd photo-tinder-desktop

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## License

MIT License - feel free to use, modify, and distribute.

---

Made with [Tauri](https://tauri.app/) + Rust + vanilla JavaScript
