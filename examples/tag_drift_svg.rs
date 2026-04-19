//! Render the 122-class × 11-release tag-drift table as an SVG heatmap.
//!
//! Each cell is coloured by how far its tag has shifted from the earliest
//! value seen in that row. Stable rows (same tag every release) are green;
//! rows with larger drift grade through amber into red. Gaps (class absent
//! that release) are dark gray.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

#[derive(Debug)]
struct Row {
    name: String,
    tags: Vec<Option<u16>>,
}

fn main() -> anyhow::Result<()> {
    let csv_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "docs/data/tag-drift-2016-2026.csv".into());
    let out_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "docs/data/tag-drift-heatmap.svg".into());

    let f = BufReader::new(File::open(&csv_path)?);
    let mut lines = f.lines();
    let header_line = lines.next().ok_or_else(|| anyhow::anyhow!("empty csv"))??;
    let releases: Vec<String> = header_line
        .split(',')
        .skip(1)
        .map(|s| s.to_string())
        .collect();

    let mut rows: Vec<Row> = Vec::new();
    for line in lines {
        let line = line?;
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != releases.len() + 1 {
            continue;
        }
        let name = parts[0].to_string();
        let tags: Vec<Option<u16>> = parts[1..]
            .iter()
            .map(|p| {
                let p = p.trim();
                if p.is_empty() {
                    None
                } else {
                    u16::from_str_radix(p.trim_start_matches("0x"), 16).ok()
                }
            })
            .collect();
        rows.push(Row { name, tags });
    }
    rows.sort_by(|a, b| {
        let a_first = a.tags.iter().find_map(|t| *t).unwrap_or(u16::MAX);
        let b_first = b.tags.iter().find_map(|t| *t).unwrap_or(u16::MAX);
        a_first.cmp(&b_first).then_with(|| a.name.cmp(&b.name))
    });

    let drift_of = |row: &Row| -> usize {
        let set: BTreeMap<u16, ()> = row.tags.iter().flatten().map(|t| (*t, ())).collect();
        set.len()
    };

    let cell_w: usize = 32;
    let cell_h: usize = 6;
    let name_col_w: usize = 260;
    let header_h: usize = 28;
    let margin: usize = 12;
    let width = name_col_w + cell_w * releases.len() + margin * 2;
    let height = header_h + cell_h * rows.len() + margin * 2 + 40;

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" font-family=\"-apple-system,Segoe UI,sans-serif\" font-size=\"10\">\n",
        w = width,
        h = height
    ));
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#0f1014\"/>\n");
    svg.push_str(&format!(
        "<text x=\"{m}\" y=\"18\" font-size=\"13\" font-weight=\"600\" fill=\"#e6e6eb\">Autodesk Revit — class-tag drift across releases (2016-2026)</text>\n",
        m = margin
    ));
    svg.push_str(&format!(
        "<text x=\"{m}\" y=\"32\" font-size=\"10\" fill=\"#8a8a95\">{r} tagged classes × {n} releases — each row one class, each column one Revit release</text>\n",
        m = margin, r = rows.len(), n = releases.len()
    ));

    let header_y = margin + header_h;
    for (i, rel) in releases.iter().enumerate() {
        let x = margin + name_col_w + i * cell_w + cell_w / 2;
        svg.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" fill=\"#9aa0ae\" font-size=\"10\">{rel}</text>\n",
            x = x, y = header_y
        ));
    }

    for (row_idx, row) in rows.iter().enumerate() {
        let y = margin + header_h + 6 + row_idx * cell_h;
        let drift = drift_of(row);
        let name_fill = if drift == 1 {
            "#5cdb7a"
        } else if drift <= 3 {
            "#d2a44a"
        } else {
            "#e06060"
        };
        svg.push_str(&format!(
            "<text x=\"{x}\" y=\"{y}\" fill=\"{fill}\" font-size=\"9\">{name}</text>\n",
            x = margin,
            y = y + 5,
            fill = name_fill,
            name = xml_escape(&row.name)
        ));

        for (col_idx, tag) in row.tags.iter().enumerate() {
            let cx = margin + name_col_w + col_idx * cell_w;
            let color = match tag {
                None => "#23252d".to_string(),
                Some(t) => {
                    let first = row.tags.iter().find_map(|t| *t).unwrap_or(*t);
                    let shift = t.saturating_sub(first);
                    if shift == 0 {
                        "#2d8040".to_string()
                    } else if shift < 10 {
                        "#8c7a38".to_string()
                    } else if shift < 25 {
                        "#b05b32".to_string()
                    } else {
                        "#b83030".to_string()
                    }
                }
            };
            svg.push_str(&format!(
                "<rect x=\"{cx}\" y=\"{y}\" width=\"{cw}\" height=\"{ch}\" fill=\"{color}\"/>\n",
                cx = cx,
                y = y,
                cw = cell_w - 2,
                ch = cell_h - 1,
                color = color
            ));
        }
    }

    let legend_y = height - 26;
    let legend_items: [(&str, &str); 5] = [
        ("#2d8040", "tag unchanged vs earliest"),
        ("#8c7a38", "shifted <10 positions"),
        ("#b05b32", "shifted 10-24 positions"),
        ("#b83030", "shifted 25 or more"),
        ("#23252d", "class absent that release"),
    ];
    let mut lx: usize = margin;
    for (color, label) in legend_items {
        svg.push_str(&format!(
            "<rect x=\"{lx}\" y=\"{y}\" width=\"10\" height=\"10\" fill=\"{color}\"/><text x=\"{tx}\" y=\"{ty}\" fill=\"#b8bac6\" font-size=\"9\">{label}</text>\n",
            lx = lx, y = legend_y, tx = lx + 14, ty = legend_y + 9
        ));
        lx += 14 + label.len() * 6 + 18;
    }

    svg.push_str("</svg>\n");

    let mut out = File::create(&out_path)?;
    out.write_all(svg.as_bytes())?;
    println!("wrote {out_path} ({width}×{height})");
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
