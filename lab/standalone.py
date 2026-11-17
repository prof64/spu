"""Wrap the built pages as complete HTML files that open straight from disk: out/html/."""
import os
here = os.path.dirname(os.path.abspath(__file__))
HEAD = '<!doctype html>\n<html lang="en">\n<head>\n<meta charset="utf-8">\n<meta name="viewport" content="width=device-width, initial-scale=1">\n<style>body{margin:0;font:14px system-ui,sans-serif}img{max-width:100%}[hidden]{display:none!important}</style>\n</head>\n<body>\n'
os.makedirs(os.path.join(here, "out/html"), exist_ok=True)
for src, dst in [("out/spu-material-lab.html", "out/html/spu-format-lab.html"), ("out/painted-or-flat.html", "out/html/painted-or-flat.html"), ("out/material-spheres.html", "out/html/material-spheres.html")]:
    if not os.path.exists(os.path.join(here, src)): continue
    page = HEAD + open(os.path.join(here, src)).read() + "\n</body>\n</html>\n"
    open(os.path.join(here, dst), "w").write(page); print(dst, len(page) // 1024, "KB")
