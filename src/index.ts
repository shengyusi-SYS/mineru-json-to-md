#!/usr/bin/env bun

import { readFileSync, existsSync, writeFileSync } from 'fs';
import { dirname, join, resolve } from 'path';

// ==================== 类型定义 ====================

interface Span {
  bbox: number[];
  type: 'text' | 'image' | 'inline_equation' | 'interline_equation' | 'table';
  content?: string;
  image_path?: string;
}

interface Line {
  bbox: number[];
  spans: Span[];
}

interface Block {
  bbox: number[];
  type: string;
  angle?: number;
  lines?: Line[];
  blocks?: Block[];
  index?: number;
  sub_type?: string;
}

interface DiscardedBlock extends Block {
  type: 'header' | 'footer' | 'page_number' | 'page_footnote' | 'aside_text';
}

interface PageInfo {
  para_blocks: Block[];
  discarded_blocks: DiscardedBlock[];
  page_size: [number, number];
  page_idx: number;
}

interface LayoutJson {
  pdf_info: PageInfo[];
  _backend?: string;
  _version_name?: string;
}

interface TocEntry {
  title: string;
  pageIdx: number;
  anchorId: string;
  level: number;
}

// ==================== 工具函数 ====================

function imageToBase64(imagePath: string, basePath: string): string {
  const fullPath = join(basePath, imagePath);
  if (!existsSync(fullPath)) {
    return '';
  }
  try {
    const imageData = readFileSync(fullPath);
    const ext = imagePath.split('.').pop()?.toLowerCase() || 'jpg';
    const mimeType = ext === 'png' ? 'image/png' : ext === 'gif' ? 'image/gif' : ext === 'webp' ? 'image/webp' : 'image/jpeg';
    return `data:${mimeType};base64,${imageData.toString('base64')}`;
  } catch {
    return '';
  }
}

function escapeHtml(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function generateAnchorId(title: string, pageIdx: number): string {
  const slug = title
    .toLowerCase()
    .replace(/[^\w\u4e00-\u9fa5]+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 50);
  return `toc-${pageIdx}-${slug || 'title'}`;
}

// ==================== 内容提取 ====================

function extractTextFromBlock(block: Block): string {
  const texts: string[] = [];

  if (block.lines) {
    for (const line of block.lines) {
      for (const span of line.spans) {
        if (span.content) {
          texts.push(span.content);
        }
      }
    }
  }

  if (block.blocks) {
    for (const subBlock of block.blocks) {
      texts.push(extractTextFromBlock(subBlock));
    }
  }

  return texts.join('');
}

function extractSpansContent(spans: Span[], basePath: string): { html: string; hasFormula: boolean } {
  let html = '';
  let hasFormula = false;

  for (const span of spans) {
    switch (span.type) {
      case 'text':
        html += escapeHtml(span.content || '');
        break;
      case 'inline_equation':
        hasFormula = true;
        html += ` $${span.content || ''}$ `;
        break;
      case 'interline_equation':
        hasFormula = true;
        html += `\n\n$$\n${span.content || ''}\n$$\n\n`;
        break;
      case 'image':
        if (span.image_path) {
          const base64 = imageToBase64(span.image_path, basePath);
          if (base64) {
            html += `<img src="${base64}" alt="image" style="max-width: 100%; height: auto;" />`;
          }
        }
        break;
    }
  }

  return { html, hasFormula };
}

// ==================== 块渲染 ====================

function renderTitle(block: Block, basePath: string, pageIdx: number): { html: string; tocEntry: TocEntry | null } {
  const text = extractTextFromBlock(block).trim();
  if (!text) return { html: '', tocEntry: null };

  const anchorId = generateAnchorId(text, pageIdx);
  const level = text.length > 20 ? 2 : 1;

  const tocEntry: TocEntry = {
    title: text,
    pageIdx: pageIdx + 1,
    anchorId,
    level,
  };

  // 使用原生 Markdown 标题 + HTML 锚点注释
  const hashes = '#'.repeat(level + 1);
  const html = `<a id="${anchorId}"></a>\n\n${hashes} ${text}\n\n`;

  return { html, tocEntry };
}

function renderText(block: Block, basePath: string): string {
  if (!block.lines) return '';

  let text = '';
  for (const line of block.lines) {
    const { html: lineHtml } = extractSpansContent(line.spans, basePath);
    text += lineHtml;
  }

  return text ? `${text}\n\n` : '';
}

function renderList(block: Block, basePath: string): string {
  if (!block.blocks || block.blocks.length === 0) return '';

  let items = '';
  for (const subBlock of block.blocks) {
    const text = extractTextFromBlock(subBlock).trim();
    if (text) {
      items += `- ${text}\n`;
    }
  }

  if (!items) return '';

  return `${items}\n`;
}

function renderImage(block: Block, basePath: string): string {
  let imageHtml = '';
  let captionHtml = '';

  if (block.blocks) {
    for (const subBlock of block.blocks) {
      if (subBlock.type === 'image_body' && subBlock.lines) {
        for (const line of subBlock.lines) {
          for (const span of line.spans) {
            if (span.type === 'image' && span.image_path) {
              const base64 = imageToBase64(span.image_path, basePath);
              if (base64) {
                imageHtml = `<img src="${base64}" alt="figure" style="max-width: 100%; height: auto; display: block; margin: 0 auto;" />`;
              }
            }
          }
        }
      } else if (subBlock.type === 'image_caption' || subBlock.type === 'image_footnote') {
        const text = extractTextFromBlock(subBlock).trim();
        if (text) {
          captionHtml += `<figcaption style="text-align: center; font-size: 0.9em; color: #666; margin-top: 0.5em;">${escapeHtml(text)}</figcaption>`;
        }
      }
    }
  }

  if (!imageHtml) return '';

  return `<figure style="margin: 1.5em 0; text-align: center;">\n${imageHtml}\n${captionHtml}\n</figure>\n\n`;
}

function renderTable(block: Block, basePath: string): string {
  let tableHtml = '';
  let captionHtml = '';
  let footnoteHtml = '';

  if (block.blocks) {
    for (const subBlock of block.blocks) {
      if (subBlock.type === 'table_body' && subBlock.lines) {
        for (const line of subBlock.lines) {
          for (const span of line.spans) {
            if (span.type === 'table' && span.image_path) {
              const base64 = imageToBase64(span.image_path, basePath);
              if (base64) {
                tableHtml = `<img src="${base64}" alt="table" style="max-width: 100%; height: auto; display: block; margin: 0 auto;" />`;
              }
            }
          }
        }
      } else if (subBlock.type === 'table_caption') {
        const text = extractTextFromBlock(subBlock).trim();
        if (text) {
          captionHtml = `<caption style="font-weight: bold; margin-bottom: 0.5em;">${escapeHtml(text)}</caption>`;
        }
      } else if (subBlock.type === 'table_footnote') {
        const text = extractTextFromBlock(subBlock).trim();
        if (text) {
          footnoteHtml = `<p style="font-size: 0.85em; color: #666; margin-top: 0.5em;">${escapeHtml(text)}</p>`;
        }
      }
    }
  }

  if (!tableHtml) return '';

  return `<div style="margin: 1.5em 0; overflow-x: auto;">\n${captionHtml}\n${tableHtml}\n${footnoteHtml}\n</div>\n\n`;
}

function renderInterlineEquation(block: Block, basePath: string): string {
  if (!block.lines) return '';

  for (const line of block.lines) {
    for (const span of line.spans) {
      if (span.type === 'interline_equation') {
        const latex = span.content || '';
        if (span.image_path) {
          const base64 = imageToBase64(span.image_path, basePath);
          if (base64) {
            return `<div style="margin: 1em 0; text-align: center;">\n<img src="${base64}" alt="equation" style="max-height: 80px;" />\n</div>\n\n`;
          }
        }
        if (latex) {
          return `\n$$\n${latex}\n$$\n\n`;
        }
      }
    }
  }

  return '';
}

function renderIndex(block: Block, basePath: string): string {
  const text = extractTextFromBlock(block).trim();
  if (!text) return '';
  return `${text}\n\n`;
}

// ==================== Discarded Blocks ====================

function renderDiscardedBlock(block: DiscardedBlock, basePath: string): string {
  const text = extractTextFromBlock(block).trim();
  if (!text) return '';

  switch (block.type) {
    case 'header':
      return `<div style="font-size: 0.8em; color: #999; border-bottom: 1px solid #eee; padding: 0.3em 0; margin-bottom: 0.5em;">${escapeHtml(text)}</div>\n`;
    case 'page_number':
      return ''; // 页码不显示
    case 'page_footnote':
      return `<div style="font-size: 0.85em; color: #666; border-top: 1px solid #ddd; padding-top: 0.5em; margin-top: 0.3em;">${escapeHtml(text)}</div>\n`;
    case 'footer':
      return `<div style="font-size: 0.8em; color: #999; border-top: 1px solid #eee; padding-top: 0.3em;">${escapeHtml(text)}</div>\n`;
    case 'aside_text':
      return `<aside style="font-size: 0.85em; color: #777; font-style: italic;">${escapeHtml(text)}</aside>\n`;
    default:
      return '';
  }
}

function categorizeDiscardedBlocks(blocks: DiscardedBlock[]): {
  headers: DiscardedBlock[];
  footnotes: DiscardedBlock[];
} {
  const headers: DiscardedBlock[] = [];
  const footnotes: DiscardedBlock[] = [];

  for (const block of blocks) {
    if (block.type === 'header' || block.type === 'page_number') {
      headers.push(block);
    } else {
      footnotes.push(block);
    }
  }

  return { headers, footnotes };
}

// ==================== 页面渲染 ====================

function renderBlock(block: Block, basePath: string, pageIdx: number): { html: string; tocEntry: TocEntry | null } {
  switch (block.type) {
    case 'title':
      return renderTitle(block, basePath, pageIdx);
    case 'text':
      return { html: renderText(block, basePath), tocEntry: null };
    case 'list':
      return { html: renderList(block, basePath), tocEntry: null };
    case 'image':
      return { html: renderImage(block, basePath), tocEntry: null };
    case 'table':
      return { html: renderTable(block, basePath), tocEntry: null };
    case 'interline_equation':
      return { html: renderInterlineEquation(block, basePath), tocEntry: null };
    case 'index':
      return { html: renderIndex(block, basePath), tocEntry: null };
    default:
      return { html: renderText(block, basePath), tocEntry: null };
  }
}

function renderPage(page: PageInfo, basePath: string): { html: string; tocEntries: TocEntry[] } {
  const tocEntries: TocEntry[] = [];
  let contentHtml = '';

  // 分类 discarded blocks
  const { headers, footnotes } = categorizeDiscardedBlocks(page.discarded_blocks);

  // 渲染页头 (headers)
  if (headers.length > 0) {
    const headerTexts = headers.map(h => extractTextFromBlock(h).trim()).filter(Boolean);
    if (headerTexts.length > 0) {
      contentHtml += `<div style="background: #fafafa; padding: 0.5em 1em; margin-bottom: 1em; border-radius: 4px; font-size: 0.85em; color: #888;">\n`;
      contentHtml += `<span>${headerTexts.join(' · ')}</span>\n`;
      contentHtml += `</div>\n\n`;
    }
  }

  // 渲染主要内容
  for (const block of page.para_blocks) {
    const { html, tocEntry } = renderBlock(block, basePath, page.page_idx);
    contentHtml += html;
    if (tocEntry) {
      tocEntries.push(tocEntry);
    }
  }

  // 渲染页脚注释 (footnotes)
  if (footnotes.length > 0) {
    const footnoteTexts = footnotes.map(f => extractTextFromBlock(f).trim()).filter(Boolean);
    if (footnoteTexts.length > 0) {
      contentHtml += `\n<div style="background: #f8f8f8; padding: 0.8em 1em; margin-top: 1.5em; border-left: 3px solid #ddd; border-radius: 0 4px 4px 0; font-size: 0.85em; color: #666;">\n`;
      contentHtml += footnoteTexts.map(t => `<p style="margin: 0.3em 0;">${escapeHtml(t)}</p>`).join('\n');
      contentHtml += `\n</div>\n\n`;
    }
  }

  return { html: contentHtml, tocEntries };
}

// ==================== 目录生成 ====================

function generateToc(tocEntries: TocEntry[]): string {
  if (tocEntries.length === 0) return '';

  let toc = '<div id="toc-top"></div>\n\n';
  return toc;
}

// ==================== 分页线 ====================

function generatePageDivider(pageNum: number): string {
  return `\n<div style="display: flex; align-items: center; margin: 2.5em 0; gap: 1em;">\n  <div style="flex: 1; height: 1px; background: #ddd;"></div>\n  <span style="color: #888; font-size: 0.85em;">第 ${pageNum} 页</span>\n  <div style="flex: 1; height: 1px; background: #ddd;"></div>\n</div>\n\n`;
}

// ==================== 主转换函数 ====================

function convertLayoutToMarkdown(layoutJson: LayoutJson, basePath: string): string {
  let markdown = '';
  const allTocEntries: TocEntry[] = [];

  // 文档头部样式
  markdown += `<style>
  body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif; }
  img { border-radius: 4px; }
  code { background: #f4f4f4; padding: 0.2em 0.4em; border-radius: 3px; font-size: 0.9em; }
  pre { background: #f8f8f8; padding: 1em; border-radius: 6px; overflow-x: auto; }
</style>\n\n`;

  // 先收集所有目录条目
  const pageContents: string[] = [];

  for (const page of layoutJson.pdf_info) {
    const { html, tocEntries } = renderPage(page, basePath);
    pageContents.push(html);
    allTocEntries.push(...tocEntries);
  }

  // 生成目录
  markdown += generateToc(allTocEntries);

  // 渲染各页内容
  for (let i = 0; i < pageContents.length; i++) {
    markdown += pageContents[i];
    markdown += generatePageDivider(i + 1);
  }

  // 文档尾部
  markdown += `\n---\n\n`;
  markdown += `*Generated by MinerU JSON to Markdown Converter*\n`;

  return markdown;
}

// ==================== CLI 入口 ====================

async function main() {
  const args = process.argv.slice(2);

  if (args.length === 0 || !args[0]) {
    console.error('Usage: bun run index.ts <path-to-json-file> [output-file]');
    console.error('Example: bun run index.ts layout.json output.md');
    process.exit(1);
  }

  const inputPath = resolve(args[0]);
  const outputPath = args[1] ? resolve(args[1]) : inputPath.replace(/\.json$/, '.md');

  if (!existsSync(inputPath)) {
    console.error(`Error: File not found: ${inputPath}`);
    process.exit(1);
  }

  console.log(`Reading: ${inputPath}`);

  try {
    const jsonContent = readFileSync(inputPath, 'utf-8');
    const layoutJson: LayoutJson = JSON.parse(jsonContent);
    const basePath = dirname(inputPath);

    console.log(`Processing ${layoutJson.pdf_info.length} pages...`);

    const markdown = convertLayoutToMarkdown(layoutJson, basePath);

    writeFileSync(outputPath, markdown, 'utf-8');
    console.log(`Output written to: ${outputPath}`);
    console.log('Done!');
  } catch (error) {
    console.error('Error:', error instanceof Error ? error.message : error);
    process.exit(1);
  }
}

main();
