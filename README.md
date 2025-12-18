# Photo Tinder

A fast, keyboard-driven desktop app for triaging and ranking large photo collections. Swipe through thousands of photos like Tinder, then rank your favorites with side-by-side comparisons.

![Photo Tinder Screenshot](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![License](https://img.shields.io/badge/License-MIT-green)

## Download

**[Download Latest Release](https://github.com/relaxis/photo-tinder-desktop/releases/latest)**

| Platform | Download |
|----------|----------|
| Windows | [Photo Tinder.msi](https://github.com/relaxis/photo-tinder-desktop/releases/latest) |
| macOS (Apple Silicon) | [Photo Tinder.dmg (arm64)](https://github.com/relaxis/photo-tinder-desktop/releases/latest) |
| macOS (Intel) | [Photo Tinder.dmg (x64)](https://github.com/relaxis/photo-tinder-desktop/releases/latest) |
| Linux | [Photo Tinder.AppImage](https://github.com/relaxis/photo-tinder-desktop/releases/latest) |
| Linux (Debian/Ubuntu) | [photo-tinder.deb](https://github.com/relaxis/photo-tinder-desktop/releases/latest) |
| Linux (Fedora/RHEL) | [photo-tinder.rpm](https://github.com/relaxis/photo-tinder-desktop/releases/latest) |

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

## Installation Notes

### Linux AppImage
```bash
chmod +x Photo\ Tinder.AppImage
./Photo\ Tinder.AppImage
```

### macOS
The app is not signed with an Apple Developer certificate. On first launch:
1. Right-click the app and select "Open"
2. Click "Open" in the security dialog

### Windows
If Windows Defender SmartScreen appears, click "More info" then "Run anyway".

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
