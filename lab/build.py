"""Assemble the pages, each one self-contained: out/spu-material-lab.html (the format lab) and
out/material-spheres.html (the material balls on their own, with no sprite data)."""
import os
here = os.path.dirname(os.path.abspath(__file__))
rd = lambda p: open(os.path.join(here, p)).read()
PARTS = [("/*SHADERS*/", "page/shaders.js"), ("/*DATA*/", "out/data.json"), ("/*APP*/", "page/app.js"), ("/*BALLS*/", "page/balls.js"), ("/*UI*/", "page/ui.js"), ("/*SPHERES*/", "page/spheres.js")]
for template, out in [("page/index.html", "out/spu-material-lab.html"), ("page/spheres.html", "out/material-spheres.html")]:
    page = rd(template)
    for tag, path in PARTS:
        if tag in page: page = page.replace(tag, rd(path))
    open(os.path.join(here, out), "w").write(page)
    print(out, len(page) // 1024, "KB")
