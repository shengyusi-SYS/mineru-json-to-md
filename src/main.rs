use base64::{Engine as _, engine::general_purpose};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

// ==================== 类型定义 ====================

#[derive(Debug, Deserialize, Clone)]
struct Span {
    bbox: Vec<f64>,
    #[serde(rename = "type")]
    span_type: String,
    content: Option<String>,
    image_path: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct Line {
    bbox: Vec<f64>,
    spans: Vec<Span>,
}

#[derive(Debug, Deserialize, Clone)]
struct Block {
    bbox: Vec<f64>,
    #[serde(rename = "type")]
    block_type: String,
    angle: Option<f64>,
    lines: Option<Vec<Line>>,
    blocks: Option<Vec<Block>>,
    index: Option<i32>,
    sub_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    para_blocks: Vec<Block>,
    discarded_blocks: Vec<Block>,
    page_size: (f64, f64),
    page_idx: usize,
}

#[derive(Debug, Deserialize)]
struct LayoutJson {
    pdf_info: Vec<PageInfo>,
    #[serde(rename = "_backend")]
    backend: Option<String>,
    #[serde(rename = "_version_name")]
    version_name: Option<String>,
}

#[derive(Debug, Clone)]
struct TocEntry {
    title: String,
    page_idx: usize,
    anchor_id: String,
    level: usize,
}

struct DiscardedBlocksCategory {
    headers: Vec<Block>,
    footnotes: Vec<Block>,
}

// ==================== 工具函数 ====================

fn image_to_base64(image_path: &str, base_path: &Path) -> Option<String> {
    let full_path = base_path.join(image_path);
    if !full_path.exists() {
        return None;
    }

    match fs::read(&full_path) {
        Ok(data) => {
            let ext = full_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("jpg")
                .to_lowercase();
            let mime_type = match ext.as_str() {
                "png" => "image/png",
                "gif" => "image/gif",
                "webp" => "image/webp",
                _ => "image/jpeg",
            };
            let encoded = general_purpose::STANDARD.encode(&data);
            Some(format!("data:{};base64,{}", mime_type, encoded))
        }
        Err(_) => None,
    }
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn generate_anchor_id(title: &str, page_idx: usize) -> String {
    let slug: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric()
                || c.is_ascii_alphabetic()
                || (c >= '\u{4e00}' && c <= '\u{9fa5}')
            {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(50)
        .collect();

    format!(
        "toc-{}-{}",
        page_idx,
        if slug.is_empty() { "title" } else { &slug }
    )
}

// ==================== 内容提取 ====================

fn extract_text_from_block(block: &Block) -> String {
    let mut texts = Vec::new();

    if let Some(lines) = &block.lines {
        for line in lines {
            for span in &line.spans {
                if let Some(content) = &span.content {
                    texts.push(content.clone());
                }
            }
        }
    }

    if let Some(blocks) = &block.blocks {
        for sub_block in blocks {
            texts.push(extract_text_from_block(sub_block));
        }
    }

    texts.join("")
}

// ==================== 行内公式和文本处理 ====================

fn render_rich_text(block: &Block) -> (String, bool) {
    let mut html = String::new();
    let mut has_formula = false;

    if let Some(lines) = &block.lines {
        for line in lines {
            for span in &line.spans {
                match span.span_type.as_str() {
                    "inline_equation" => {
                        has_formula = true;
                        if let Some(content) = &span.content {
                            html.push_str(&format!("${}", content));
                            html.push('$');
                        }
                    }
                    "text" => {
                        if let Some(content) = &span.content {
                            html.push_str(content);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    (html, has_formula)
}

// ==================== 块渲染 ====================

fn render_title(block: &Block, page_idx: usize) -> (String, Option<TocEntry>) {
    let text = extract_text_from_block(block).trim().to_string();
    if text.is_empty() {
        return (String::new(), None);
    }

    let anchor_id = generate_anchor_id(&text, page_idx);
    let level = if text.chars().count() > 20 { 2 } else { 1 };

    let toc_entry = TocEntry {
        title: text.clone(),
        page_idx: page_idx + 1,
        anchor_id: anchor_id.clone(),
        level,
    };

    // 使用原生 Markdown 标题 + HTML 锚点作为隐藏导航元素
    let heading_prefix = if level == 1 { "##" } else { "###" };
    let html = format!(
        "<a id=\"{}\"></a>\n{} {}\n\n",
        anchor_id, heading_prefix, text
    );

    (html, Some(toc_entry))
}

fn render_text(block: &Block) -> String {
    let (text, _has_formula) = render_rich_text(block);
    if text.trim().is_empty() {
        return String::new();
    }
    format!("{}\n\n", text.trim())
}

fn render_list(block: &Block) -> String {
    if let Some(blocks) = &block.blocks {
        let mut items = Vec::new();
        for sub_block in blocks {
            if sub_block.block_type == "list_item" {
                let text = extract_text_from_block(sub_block).trim().to_string();
                if !text.is_empty() {
                    items.push(format!("- {}", text));
                }
            }
        }
        if !items.is_empty() {
            return format!("{}\n\n", items.join("\n"));
        }
    }
    String::new()
}

fn render_image(block: &Block, base_path: &Path) -> String {
    let mut image_html = String::new();
    let mut caption_html = String::new();

    if let Some(blocks) = &block.blocks {
        for sub_block in blocks {
            if sub_block.block_type == "image_body" {
                if let Some(lines) = &sub_block.lines {
                    for line in lines {
                        for span in &line.spans {
                            if span.span_type == "image" {
                                if let Some(image_path) = &span.image_path {
                                    if let Some(base64) = image_to_base64(image_path, base_path) {
                                        image_html = format!(
                                            "<img src=\"{}\" alt=\"figure\" style=\"max-width: 100%; height: auto; display: block; margin: 0 auto;\" />",
                                            base64
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            } else if sub_block.block_type == "image_caption"
                || sub_block.block_type == "image_footnote"
            {
                let text = extract_text_from_block(sub_block).trim().to_string();
                if !text.is_empty() {
                    caption_html.push_str(&format!(
                        "<figcaption style=\"text-align: center; font-size: 0.9em; color: #666; margin-top: 0.5em;\">{}</figcaption>",
                        escape_html(&text)
                    ));
                }
            }
        }
    }

    if image_html.is_empty() {
        return String::new();
    }

    format!(
        "<figure style=\"margin: 1.5em 0; text-align: center;\">\n{}\n{}\n</figure>\n\n",
        image_html, caption_html
    )
}

fn render_table(block: &Block, base_path: &Path) -> String {
    let mut table_html = String::new();
    let mut caption_html = String::new();
    let mut footnote_html = String::new();

    if let Some(blocks) = &block.blocks {
        for sub_block in blocks {
            if sub_block.block_type == "table_body" {
                if let Some(lines) = &sub_block.lines {
                    for line in lines {
                        for span in &line.spans {
                            if span.span_type == "table" {
                                if let Some(image_path) = &span.image_path {
                                    if let Some(base64) = image_to_base64(image_path, base_path) {
                                        table_html = format!(
                                            "<img src=\"{}\" alt=\"table\" style=\"max-width: 100%; height: auto; display: block; margin: 0 auto;\" />",
                                            base64
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            } else if sub_block.block_type == "table_caption" {
                let text = extract_text_from_block(sub_block).trim().to_string();
                if !text.is_empty() {
                    caption_html = format!(
                        "<caption style=\"font-weight: bold; margin-bottom: 0.5em;\">{}</caption>",
                        escape_html(&text)
                    );
                }
            } else if sub_block.block_type == "table_footnote" {
                let text = extract_text_from_block(sub_block).trim().to_string();
                if !text.is_empty() {
                    footnote_html = format!(
                        "<p style=\"font-size: 0.85em; color: #666; margin-top: 0.5em;\">{}</p>",
                        escape_html(&text)
                    );
                }
            }
        }
    }

    if table_html.is_empty() {
        return String::new();
    }

    format!(
        "<div style=\"margin: 1.5em 0; overflow-x: auto;\">\n{}\n{}\n{}\n</div>\n\n",
        caption_html, table_html, footnote_html
    )
}

fn render_interline_equation(block: &Block, base_path: &Path) -> String {
    if let Some(lines) = &block.lines {
        for line in lines {
            for span in &line.spans {
                if span.span_type == "interline_equation" {
                    if let Some(image_path) = &span.image_path {
                        if let Some(base64) = image_to_base64(image_path, base_path) {
                            return format!(
                                "<div style=\"margin: 1em 0; text-align: center;\">\n<img src=\"{}\" alt=\"equation\" style=\"max-height: 80px;\" />\n</div>\n\n",
                                base64
                            );
                        }
                    }
                    if let Some(latex) = &span.content {
                        return format!("\n$$\n{}\n$$\n\n", latex);
                    }
                }
            }
        }
    }
    String::new()
}

fn render_index(block: &Block) -> String {
    let text = extract_text_from_block(block).trim().to_string();
    if text.is_empty() {
        return String::new();
    }
    format!("{}\n\n", text)
}

// ==================== Discarded Blocks ====================

fn categorize_discarded_blocks(blocks: &[Block]) -> DiscardedBlocksCategory {
    let mut headers = Vec::new();
    let mut footnotes = Vec::new();

    for block in blocks {
        match block.block_type.as_str() {
            "header" => headers.push(block.clone()),
            "page_footnote" => footnotes.push(block.clone()),
            _ => {}
        }
    }

    DiscardedBlocksCategory { headers, footnotes }
}

fn render_discarded_headers(blocks: &[Block]) -> String {
    blocks
        .iter()
        .filter_map(|block| {
            let text = extract_text_from_block(block).trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(format!(
                    "<div style=\"background: #fafafa; padding: 0.5em 1em; margin-bottom: 1em; border-radius: 4px; font-size: 0.85em; color: #888;\">\n<span>{}</span>\n</div>\n\n",
                    escape_html(&text)
                ))
            }
        })
        .collect()
}

fn render_discarded_footnotes(blocks: &[Block]) -> String {
    if blocks.is_empty() {
        return String::new();
    }

    let mut html = String::from(
        "<div style=\"background: #f8f8f8; padding: 0.8em 1em; margin-top: 1.5em; border-left: 3px solid #ddd; border-radius: 0 4px 4px 0; font-size: 0.85em; color: #666;\">\n",
    );

    for block in blocks {
        let text = extract_text_from_block(block).trim().to_string();
        if !text.is_empty() {
            html.push_str(&format!(
                "<p style=\"margin: 0.3em 0;\">{}</p>\n",
                escape_html(&text)
            ));
        }
    }

    html.push_str("</div>\n\n");
    html
}

// ==================== 页面渲染 ====================

fn render_block(block: &Block, base_path: &Path, page_idx: usize) -> (String, Option<TocEntry>) {
    match block.block_type.as_str() {
        "title" => render_title(block, page_idx),
        "text" => (render_text(block), None),
        "list" => (render_list(block), None),
        "image" => (render_image(block, base_path), None),
        "table" => (render_table(block, base_path), None),
        "interline_equation" => (render_interline_equation(block, base_path), None),
        "index" => (render_index(block), None),
        _ => (render_text(block), None),
    }
}

fn render_page(page: &PageInfo, base_path: &Path) -> (String, Vec<TocEntry>) {
    let mut toc_entries = Vec::new();
    let mut content_html = String::new();

    // 分类 discarded blocks
    let categorized = categorize_discarded_blocks(&page.discarded_blocks);

    // 添加页眉
    content_html.push_str(&render_discarded_headers(&categorized.headers));

    // 渲染主要内容块
    for block in &page.para_blocks {
        let (html, toc_entry) = render_block(block, base_path, page.page_idx);
        content_html.push_str(&html);
        if let Some(entry) = toc_entry {
            toc_entries.push(entry);
        }
    }

    // 添加脚注
    content_html.push_str(&render_discarded_footnotes(&categorized.footnotes));

    (content_html, toc_entries)
}

// ==================== 目录生成 ====================

fn generate_toc(toc_entries: &[TocEntry]) -> String {
    if toc_entries.is_empty() {
        return String::new();
    }

    let toc = String::from("<div id=\"toc-top\"></div>\n\n");
    toc
}

// ==================== 分页线 ====================

fn generate_page_divider(page_num: usize) -> String {
    format!(
        "\n<div style=\"display: flex; align-items: center; margin: 2.5em 0; gap: 1em;\">\n  <div style=\"flex: 1; height: 1px; background: #ddd;\"></div>\n  <span style=\"color: #888; font-size: 0.85em;\">第 {} 页</span>\n  <div style=\"flex: 1; height: 1px; background: #ddd;\"></div>\n</div>\n\n",
        page_num
    )
}

// ==================== 主转换函数 ====================

fn convert_layout_to_markdown(layout_json: &LayoutJson, base_path: &Path) -> String {
    let mut markdown = String::new();
    let mut all_toc_entries = Vec::new();

    // 文档头部样式
    markdown.push_str(
        "<style>\n  body { font-family: -apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif; }\n  img { border-radius: 4px; }\n  code { background: #f4f4f4; padding: 0.2em 0.4em; border-radius: 3px; font-size: 0.9em; }\n  pre { background: #f8f8f8; padding: 1em; border-radius: 6px; overflow-x: auto; }\n</style>\n\n",
    );

    // 先收集所有目录条目
    let mut page_contents = Vec::new();

    for page in &layout_json.pdf_info {
        let (html, toc_entries) = render_page(page, base_path);
        page_contents.push(html);
        all_toc_entries.extend(toc_entries);
    }

    // 生成目录
    markdown.push_str(&generate_toc(&all_toc_entries));

    // 添加分隔线
    markdown.push_str(
        "<hr style=\"border: none; height: 1px; background: #ddd; margin: 2em 0;\" />\n\n",
    );

    // 渲染各页内容
    for (i, content) in page_contents.iter().enumerate() {
        markdown.push_str(content);
        markdown.push_str(&generate_page_divider(i + 1));
    }

    // 文档尾部
    markdown.push_str(
        "\n<hr style=\"border: none; height: 1px; background: #ddd; margin: 3em 0;\" />\n",
    );
    markdown.push_str(
        "<div style=\"text-align: center; color: #999; font-size: 0.85em; padding: 1em 0;\">\n",
    );
    markdown.push_str("Generated by MinerU JSON to Markdown Converter\n");
    markdown.push_str("</div>\n");

    markdown
}

// ==================== CLI 入口 ====================

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: mineru-json-to-md <path-to-json-file> [output-file]");
        eprintln!("Example: mineru-json-to-md layout.json output.md");
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[1]);
    let output_path = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        input_path.with_extension("md")
    };

    if !input_path.exists() {
        eprintln!("Error: File not found: {}", input_path.display());
        std::process::exit(1);
    }

    println!("Reading: {}", input_path.display());

    let json_content = match fs::read_to_string(&input_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    let layout_json: LayoutJson = match serde_json::from_str(&json_content) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error parsing JSON: {}", e);
            std::process::exit(1);
        }
    };

    let base_path = input_path.parent().unwrap_or(Path::new("."));

    println!("Processing {} pages...", layout_json.pdf_info.len());

    let markdown = convert_layout_to_markdown(&layout_json, base_path);

    match fs::write(&output_path, markdown) {
        Ok(_) => {
            println!("Output written to: {}", output_path.display());
            println!("Done!");
        }
        Err(e) => {
            eprintln!("Error writing output: {}", e);
            std::process::exit(1);
        }
    }
}
