# BBOW Browser 🌐

A modern terminal-based web browser that intelligently summarizes web content using AI. BBOW (pronounced "bow") fetches web pages, extracts meaningful content, and presents beautifully formatted summaries with easy link navigation—all from your terminal.

## ✨ Features

- **AI-Powered Summaries**: Automatically generates clean, structured summaries using GPT-4o-mini
- **Beautiful Markdown Rendering**: Rich text formatting with headers, bold, italic, code blocks, and bullet points
- **Smart Link Extraction**: Filters out noise and presents only meaningful navigation options
- **Intuitive TUI Interface**: Professional terminal interface built with Ratatui
- **Real-time Progress Tracking**: Visual progress bar showing fetch, parse, and AI processing stages
- **Navigation History**: Full browsing history with forward/back functionality
- **Responsive Layout**: Adapts to any terminal size with optimized 80/20 content-to-links ratio
- **Keyboard-Driven**: Efficient navigation without needing a mouse

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- OpenAI API key (get one from [OpenAI](https://platform.openai.com/api-keys))

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/bbow.git
   cd bbow
   ```

2. **Set up your OpenAI API key**
   ```bash
   export OPENAI_API_KEY="your-api-key-here"
   # Or create a .env file with: OPENAI_API_KEY=your-api-key-here
   ```

3. **Build and run**
   ```bash
   cargo build --release
   ./target/release/bbow
   ```

## 🎯 Usage

### Navigation

| Key | Action |
|-----|--------|
| `g` | Enter URL |
| `↑↓` | Scroll content |
| `Shift+↑↓` | Select links |
| `Enter` | Follow selected link |
| `1-9` | Follow link by number |
| `b` | Go back |
| `f` | Go forward |
| `h` | View history |
| `r` | Refresh page |
| `q` | Quit |

### Getting Started

1. Launch BBOW: `./target/release/bbow`
2. Press `g` to enter a URL
3. Type any website (e.g., `news.ycombinator.com`)
4. Watch the progress bar as BBOW fetches and processes the content
5. Read the AI-generated summary and use `Shift+↑↓` to navigate links

## 🏗️ Architecture

BBOW is built with a clean, modular architecture:

```
src/
├── main.rs          # Application entry point
├── browser.rs       # Core browser logic and state management
├── client.rs        # HTTP client for web requests
├── extractor.rs     # HTML text extraction and cleaning
├── openai.rs        # OpenAI API integration
├── links.rs         # Smart link extraction and filtering
├── ui.rs            # Terminal user interface (TUI)
└── history.rs       # Navigation history management
```

## 🔧 Configuration

### Environment Variables

- `OPENAI_API_KEY` - Your OpenAI API key (required)

### Customization

The following constants can be modified in the source code:

**OpenAI Settings** (`src/openai.rs`):
- `OPENAI_MODEL` - AI model to use (default: "gpt-4o-mini")
- `MAX_TOKENS` - Maximum response length (default: 500)
- `TEMPERATURE` - AI creativity level (default: 0.3)

**Network Settings** (`src/client.rs`):
- `REQUEST_TIMEOUT_SECS` - HTTP timeout (default: 30)
- `MAX_REDIRECTS` - Maximum HTTP redirects (default: 5)

**Link Filtering** (`src/links.rs`):
- `MIN_LINK_TEXT_LENGTH` - Minimum link text length (default: 2)
- `MAX_URL_LENGTH` - Maximum URL length (default: 200)

## 🎨 Features in Detail

### AI Summaries
BBOW uses GPT-4o-mini to create concise, well-structured summaries of web content. The AI is specifically prompted to:
- Use proper markdown formatting
- Create clear section headers
- Highlight important information in bold
- Present lists as bullet points
- Maintain readability and structure

### Smart Link Filtering
The link extraction system automatically filters out:
- Navigation noise (ads, social media buttons, etc.)
- Empty or meaningless links
- Image and media file links
- Tracking and analytics URLs
- Duplicate destinations

### Progress Tracking
Real-time progress indication shows:
1. **25%** - Fetching HTML content
2. **50%** - Extracting text content
3. **75%** - Processing page structure
4. **90%** - Generating AI summary
5. **100%** - Complete!

## 🤝 Contributing

We welcome contributions! Please feel free to:

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/amazing-feature`
3. **Commit your changes**: `git commit -m 'Add amazing feature'`
4. **Push to the branch**: `git push origin feature/amazing-feature`
5. **Open a Pull Request**

### Development Setup

```bash
# Clone and setup
git clone https://github.com/yourusername/bbow.git
cd bbow

# Run in development mode
cargo run

# Run tests
cargo test

# Check code quality
cargo clippy
cargo fmt
```

## 📋 System Requirements

- **OS**: Linux, macOS, Windows
- **Terminal**: Any terminal with Unicode support
- **Rust**: 1.70 or later
- **Network**: Internet connection for web requests and OpenAI API

## ⚡ Performance

BBOW is optimized for speed and efficiency:
- **Fast startup**: Minimal dependencies, quick initialization
- **Efficient parsing**: Optimized HTML processing and text extraction
- **Smart caching**: Avoids redundant processing where possible
- **Low memory**: Minimal memory footprint for terminal usage

## 🛡️ Privacy & Security

- **API Key Security**: Your OpenAI API key is only used for summary generation
- **No Data Storage**: BBOW doesn't store or cache web content permanently
- **Local Processing**: All text extraction and processing happens locally
- **Secure Requests**: HTTPS-only web requests with proper SSL verification

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Ratatui** - Excellent TUI framework for Rust
- **Tokio** - Async runtime for Rust
- **Reqwest** - HTTP client library
- **Scraper** - HTML parsing and CSS selector engine
- **OpenAI** - AI-powered content summarization

## 🔮 Roadmap

- [ ] Bookmark system
- [ ] Search within summaries
- [ ] Custom AI prompts
- [ ] Offline mode for cached content
- [ ] Plugin system for custom processors
- [ ] Image and media preview support

---

**Made with ❤️ and Rust** | *Happy browsing in your terminal!* 🦀