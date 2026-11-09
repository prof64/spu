"""Local-only preview; no uploads, telemetry, or external service dependencies."""
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
import argparse

parser = argparse.ArgumentParser()
parser.add_argument("--port", type=int, default=8767)
args = parser.parse_args()
root = Path(__file__).resolve().parent.parent / "web"

class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(root), **kwargs)

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        super().end_headers()

print(f"Sprite Lab: http://127.0.0.1:{args.port}", flush=True)
ThreadingHTTPServer(("127.0.0.1", args.port), Handler).serve_forever()
