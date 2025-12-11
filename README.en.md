# MinerU JSON to Markdown

> **⚠️ Disclaimer**  
> All content in this project is AI-generated and has only been partially tested by the author. Actual usage effectiveness is subject to your own experience, and full functionality is not guaranteed in all scenarios.

Convert MinerU's `layout.json` files to well-formatted Markdown documents with embedded images, native headings, formulas, and rich formatting support.

[中文](README.md) | English

## 📺 Demo

<img src="./example.png" width = "300"  alt="example" align=center />

## ✨ Features

### Core Functionality

- 📄 **Complete Block Type Support**
  - `title` - Headings (native Markdown `##` `###` format with HTML anchors)
  - `text` - Plain text (native Markdown format)
  - `list` - Lists (native Markdown list syntax)
  - `image` - Images (Base64 inline embedding)
  - `table` - Tables (HTML rendering)
  - `interline_equation` - Display equations (LaTeX or images)
  - `index` - Index blocks

### Advanced Features

- 📊 **Clean Pagination** - Page numbers with separators at the end of each page (display only, no navigation)
- 🎨 **Clean Styling** - Light single-color background design (#fafafa, #f8f8f8)
- 🔢 **Inline Formulas** - Preserves `$...$` format for inline math equations
- 📝 **Header/Footer Processing**
  - `header` - Page headers (light styled display)
  - `page_footnote` - Page footnotes (blockquote style)
  - `page_number` - Page numbers (automatically filtered)
- 🖼️ **Image Processing** - Auto-convert to Base64 and inline into Markdown
- 📐 **Table Support** - Preserves table captions, footnotes, and complete information
- ✏️ **Native Markdown First** - Headings use native `##` syntax with HTML anchors as hidden navigation elements

## 📦 Installation & Usage

### TypeScript Version (Bun)

#### Prerequisites

- [Bun](https://bun.sh) >= 1.0.0

#### Install Dependencies

```bash
bun install
```

#### Usage

```bash
# Basic usage
bun run src/index.ts <layout.json path> [output-file.md]

# Example
bun run src/index.ts layout.json output.md

# Simplified (auto-infer output filename)
bun run src/index.ts layout.json
```

### Rust Version

#### Prerequisites

- Rust >= 1.70 (only required for development/compilation)

#### Build

```bash
cargo build --release
```

The compiled executable is located in the `target/release/` directory:

- Windows: `mineru-json-to-md.exe`
- Linux/macOS: `mineru-json-to-md`

#### Usage

**Method 1: Command Line**

```bash
# Windows
.\target\release\mineru-json-to-md.exe layout.json output.md

# Linux/macOS
./target/release/mineru-json-to-md layout.json output.md

# Simplified (auto-infer output filename)
.\target\release\mineru-json-to-md.exe layout.json
```

**Method 2: Drag & Drop (Recommended)**

- Simply drag and drop `layout.json` file onto `mineru-json-to-md.exe`
- Automatically generates same-name `.md` file in the same directory

**Deployment**

The compiled exe file can run standalone without requiring Rust installation:

1. Copy `target/release/mineru-json-to-md.exe` to any directory
2. Double-click to run or drag JSON files onto the exe

#### Install to System (Optional)

```bash
cargo install --path .
mineru-json-to-md layout.json output.md
```

## 📖 Input Format

### MinerU Layout JSON Structure

```json
{
  "pdf_info": [
    {
      "page_idx": 0,
      "page_size": [595.0, 842.0],
      "para_blocks": [
        {
          "type": "title",
          "bbox": [x, y, x, y],
          "lines": [
            {
              "spans": [
                {
                  "type": "text",
                  "content": "Title text"
                }
              ]
            }
          ]
        }
      ],
      "discarded_blocks": [
        {
          "type": "header",
          "lines": [...]
        }
      ]
    }
  ]
}
```

## 📝 Output Format

Generated Markdown document includes:

1. **Document Styles** - Embedded CSS styles for proper rendering in HTML-compatible Markdown renderers
2. **Anchor Placeholder** - Top anchor `<div id="toc-top"></div>` (reserved for extension)
3. **Separator Line** - Horizontal line at document beginning
4. **Main Content** - All content blocks ordered by page number
5. **Page Dividers** - Clean page number display at the end of each page (`--- Page N ---`)
6. **Native Headings** - Uses `##` or `###` Markdown headings with `<a id="..."></a>` anchors

### Sample Output Structure

```markdown
<style>...</style>

<div id="toc-top"></div>

---

<a id="toc-0-title1"></a>

## Title 1

Main content...

---

## Page 1

<a id="toc-5-title2"></a>

## Title 2

More content...

---

## Page 2
```

## 🎯 Use Cases

- ✅ Convert MinerU OCR recognition results to readable Markdown documents
- ✅ Generate clean and concise academic papers/book documents
- ✅ Preserve complex elements like images, tables, and formulas from original documents
- ✅ Create Markdown documents with embedded images (no external files needed)
- ✅ Prioritize native Markdown syntax for better portability

## 📂 Project Structure

```
mineru-json-to-md/
├── src/
│   ├── index.ts         # TypeScript version main file
│   └── main.rs          # Rust version main file
├── package.json         # Bun/Node.js configuration
├── Cargo.toml           # Rust project configuration
├── tsconfig.json        # TypeScript configuration
├── README.md            # Documentation (Chinese)
├── README.en.md         # Documentation (English)
├── layout.json          # Sample input file
└── example.png          # Example screenshot
```

## 🔧 Tech Stack

### TypeScript Version

- **Runtime**: Bun
- **Language**: TypeScript
- **Dependencies**: Node.js built-in modules (fs, path)

### Rust Version

- **Language**: Rust (Edition 2024)
- **Dependencies**:
  - `serde` - JSON serialization/deserialization
  - `serde_json` - JSON parsing
  - `base64` - Base64 encoding

## 🤝 Contributing

Issues and Pull Requests are welcome!

## 📄 License

MIT License

## 🙏 Acknowledgments

This tool is designed based on the output format of the [MinerU](https://github.com/opendatalab/MinerU) project.

## 📞 Contact

For questions or suggestions, please contact via GitHub Issues.
