import os
import re

readme_path = r"C:\Pato\Nusantara-Rust-Suite\kurung\README.md"
preview_path = r"C:\Pato\Nusantara-Rust-Suite\kurung\docs\preview.html"

with open(readme_path, "r", encoding="utf-8") as f:
    text = f.read()

# Replace mermaid blocks
def replace_mermaid(match):
    code = match.group(1).strip()
    return f'<div class="mermaid">\n{code}\n</div>'

text = re.sub(r"```mermaid\n(.*?)\n```", replace_mermaid, text, flags=re.DOTALL)

# Fix relative image paths for docs/preview.html
text = text.replace("docs/assets/", "assets/")

try:
    import markdown
    body = markdown.markdown(text, extensions=["tables", "fenced_code"])
except ImportError:
    body = f"<pre>{text}</pre>"

html = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Kurung README Live Preview</title>
<script type="module">
  import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
  mermaid.initialize({{ startOnLoad: true, theme: 'dark' }});
</script>
<style>
  body {{
    background: #0d1117;
    color: #c9d1d9;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
    line-height: 1.6;
    padding: 32px 24px;
    max-width: 860px;
    margin: 0 auto;
  }}
  img {{ max-width: 100%; border-radius: 6px; }}
  table {{ border-collapse: collapse; width: 100%; margin: 20px 0; font-size: 13px; }}
  th, td {{ border: 1px solid #30363d; padding: 10px 14px; text-align: left; }}
  th {{ background: #161b22; color: #f0f6fc; font-weight: 600; }}
  tr:nth-child(even) {{ background: #161b22; }}
  code {{ background: rgba(110,118,129,0.3); padding: 2px 6px; border-radius: 4px; font-family: ui-monospace, SFMono-Regular, monospace; font-size: 13px; }}
  pre {{ background: #161b22; border: 1px solid #30363d; border-radius: 6px; padding: 16px; overflow-x: auto; }}
  pre code {{ background: none; padding: 0; }}
  a {{ color: #58a6ff; text-decoration: none; }}
  a:hover {{ text-decoration: underline; }}
  hr {{ border: 0; border-top: 1px solid #30363d; margin: 28px 0; }}
  .mermaid {{ background: #161b22; padding: 16px; border-radius: 8px; border: 1px solid #30363d; margin: 20px 0; }}
  h1, h2, h3 {{ color: #f0f6fc; border-bottom: 1px solid #21262d; padding-bottom: 8px; margin-top: 24px; }}
</style>
</head>
<body>
{body}
</body>
</html>"""

with open(preview_path, "w", encoding="utf-8") as f:
    f.write(html)

print("Preview HTML generated successfully at:", preview_path)

