# The demo

`record.py` makes `docs/demo.gif`, the recording at the top of the main README.

It opens a made-up invoice in Safari, in the background. It starts the bar with two tasks: read the invoice, then pay it. The second task hits the money gate. The script records only three windows: the bar, the invoice page, and the gate panel. Nothing else on the screen is captured.

```
make install
uv run demo/record.py
```

It needs `ffmpeg` and `uv`. Leave the gate panel alone while it records, so the recording shows the refusal.
