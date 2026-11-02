import os
import subprocess
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

OUTPUT_DIR = Path("C:/Pato/Nusantara-Rust-Suite/kurung/docs/assets")
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
FRAMES_DIR = OUTPUT_DIR / "story_frames"
FRAMES_DIR.mkdir(parents=True, exist_ok=True)

WIDTH, HEIGHT = 1000, 620
BG_COLOR = (11, 15, 25)
PANEL_BG = (17, 24, 39)
BORDER_COLOR = (31, 41, 55)
TEXT_WHITE = (243, 244, 246)
TEXT_MUTED = (156, 163, 175)
COLOR_BLUE = (56, 189, 248)
COLOR_GREEN = (16, 185, 129)
COLOR_RED = (244, 63, 94)
COLOR_YELLOW = (251, 191, 36)

try:
    FONT_BIG = ImageFont.truetype("consola.ttf", 22)
    FONT_TITLE = ImageFont.truetype("consolab.ttf", 18)
    FONT_TEXT = ImageFont.truetype("consola.ttf", 15)
    FONT_BOLD = ImageFont.truetype("consolab.ttf", 15)
    FONT_SMALL = ImageFont.truetype("consola.ttf", 13)
except Exception:
    FONT_BIG = ImageFont.load_default()
    FONT_TITLE = ImageFont.load_default()
    FONT_TEXT = ImageFont.load_default()
    FONT_BOLD = ImageFont.load_default()
    FONT_SMALL = ImageFont.load_default()

def draw_canvas(draw, act_badge, act_title):
    # Background
    draw.rectangle([0, 0, WIDTH, HEIGHT], fill=BG_COLOR)
    # Outer Panel
    draw.rectangle([20, 20, WIDTH - 20, HEIGHT - 20], fill=PANEL_BG, outline=BORDER_COLOR, width=2)
    
    # Top Bar
    draw.rectangle([20, 20, WIDTH - 20, 70], fill=(24, 31, 47), outline=BORDER_COLOR, width=1)
    # Window Buttons
    draw.ellipse([35, 40, 47, 52], fill=(244, 63, 94))
    draw.ellipse([55, 40, 67, 52], fill=(251, 191, 36))
    draw.ellipse([75, 40, 87, 52], fill=(16, 185, 129))
    
    # Header Title
    draw.text((105, 37), "KURUNG DEFENSE ENGINE — MECOB ARCHITECTURE", fill=COLOR_BLUE, font=FONT_TITLE)

    # Act Badge Bar
    draw.rectangle([40, 85, WIDTH - 40, 130], fill=(15, 23, 42), outline=BORDER_COLOR, width=1)
    draw.rectangle([48, 93, 160, 122], fill=(2, 132, 199))
    draw.text((58, 98), act_badge, fill=(255, 255, 255), font=FONT_BOLD)
    draw.text((175, 98), act_title, fill=TEXT_WHITE, font=FONT_BOLD)

def create_storyboard():
    # 6 clear story acts with 6 seconds (60 frames at 10 fps) per act so viewers can read comfortably
    acts = [
        {
            "badge": "OVERVIEW",
            "title": "Why Autonomous AI Agents Need a Sandbox",
            "frames": 50,
            "box1_title": "THE PROBLEM (WITHOUT KURUNG)",
            "box1_color": COLOR_RED,
            "box1_lines": [
                "When AI agents (Claude, Antigravity, OpenClaw) write and run code,",
                "they execute unpredictable scripts directly on your computer:",
                "- Buggy infinite loops freeze your CPU cores.",
                "- Memory leaks crash your active trading bots.",
                "- File escapes risk reading your passwords and private keys."
            ],
            "box2_title": "THE SOLUTION (KURUNG ENGINE)",
            "box2_color": COLOR_GREEN,
            "box2_lines": [
                "Kurung puts untrusted AI code inside a bulletproof WebAssembly jail:",
                "- 100% Pure Rust (#![forbid(unsafe_code)]).",
                "- Sub-millisecond cold start: 787 microseconds (0.78 ms).",
                "- Exact instruction fuel budgeting + RAM hard ceiling.",
                "- Complete isolation from your Windows disks and files."
            ]
        },
        {
            "badge": "SCENE 1 / 4",
            "title": "Threat 1: The Infinite Loop (CPU Core Hijack)",
            "frames": 60,
            "box1_title": "AI AGENT SYNTHESIZES BROKEN CODE",
            "box1_color": COLOR_RED,
            "box1_lines": [
                "Agent creates an endless loop: (loop $inf (br $inf))",
                "",
                "[WITHOUT KURUNG]:",
                "- 100% CPU core utilization immediately.",
                "- Laptop fans scream, system freezes.",
                "- Requires manual Windows Task Manager kill."
            ],
            "box2_title": "KURUNG DETERMINISTIC INTERCEPTION",
            "box2_color": COLOR_GREEN,
            "box2_lines": [
                "Instruction Meter: 10,000... 30,000... 50,000 [FUEL EXHAUSTED]",
                "",
                "[WITH KURUNG]:",
                "[SECURE] OutOfFuel Trap triggered at EXACT instruction #50,000.",
                "[RESULT] Stopped in 412 microseconds (0.0004 sec).",
                "[STATUS] Host CPU: 0% impact | System perfectly safe."
            ]
        },
        {
            "badge": "SCENE 2 / 4",
            "title": "Threat 2: The Memory Bomb (Out-of-Memory Attack)",
            "frames": 60,
            "box1_title": "AI AGENT REQUESTS MASSIVE RAM",
            "box1_color": COLOR_RED,
            "box1_lines": [
                "Agent attempts to allocate 10,000 WASM pages (655 MB RAM):",
                "(memory.grow 10000)",
                "",
                "[WITHOUT KURUNG]:",
                "- System paging file explodes.",
                "- Active trading bots and browser crash due to OS OOM."
            ],
            "box2_title": "KURUNG LINEAR MEMORY CEILING",
            "box2_color": COLOR_GREEN,
            "box2_lines": [
                "Configured Sandbox Policy: Max 16 MiB RAM Ceiling.",
                "",
                "[WITH KURUNG]:",
                "[SECURE] HostContext::memory_growing() intercepts request.",
                "[RESULT] Allocation denied -> returns -1 to guest code.",
                "[STATUS] Host RAM used: 64 KiB only | Zero OS memory thrashing."
            ]
        },
        {
            "badge": "SCENE 3 / 4",
            "title": "Threat 3: The Secret Vault Leak (Path Traversal)",
            "frames": 60,
            "box1_title": "AI AGENT TRIES READING PRIVATE DISK",
            "box1_color": COLOR_RED,
            "box1_lines": [
                "Agent tries to read Windows system or private credentials:",
                "(vfs_read \"../../Windows/System32\" or \".env\")",
                "",
                "[WITHOUT KURUNG]:",
                "- Private API keys, tokens, and broker secrets are exposed."
            ],
            "box2_title": "KURUNG ISOLATED IN-MEMORY VFS",
            "box2_color": COLOR_GREEN,
            "box2_lines": [
                "Virtual Filesystem Policy: Default-Deny In-Memory BTreeMap.",
                "",
                "[WITH KURUNG]:",
                "[SECURE] Sanitizer detects forbidden path components ('..', '\\').",
                "[RESULT] Access Denied (-2). Agent cannot see your C:\\ drive.",
                "[STATUS] Zero Host Disk Leaks | Complete filesystem confinement."
            ]
        },
        {
            "badge": "SCENE 4 / 4",
            "title": "Valid Execution: Safe Compute & Cryptographic Receipt",
            "frames": 60,
            "box1_title": "VALID AGENT TOOL EXECUTION",
            "box1_color": COLOR_BLUE,
            "box1_lines": [
                "Agent computes math tool: sum of cubes (1^3 + ... + 10^3)",
                "Payload: \"ANTIGRAVITY_AGENT_OK\"",
                "",
                "Execution Return Value: 3025",
                "Execution Duration    : 321 microseconds (0.32 ms)",
                "Instruction Fuel Used : 64 instructions only"
            ],
            "box2_title": "CRYPTOGRAPHIC PROOF OF EXECUTION",
            "box2_color": COLOR_GREEN,
            "box2_lines": [
                "Kurung issues an immutable SHA-256 tamper-evident receipt:",
                "",
                "Module Hash : c53fa1151bb41d12b0827bb39f79afae61070...",
                "Output Hash : f2c7e2fd4898808a252937e93ca088a78110...",
                "Status      : OK | Proof of Safe Deterministic Compute",
                "[MECOB] Audit trail logged to disk with zero doubt."
            ]
        },
        {
            "badge": "SUMMARY",
            "title": "Kurung: The Speed of Native Code + The Safety of a Jail",
            "frames": 50,
            "box1_title": "SPEED COMPARISON",
            "box1_color": COLOR_BLUE,
            "box1_lines": [
                "- Docker Container Cold Start : 300 ms - 1,500 ms (Too slow for AI)",
                "- MicroVM (Firecracker/gVisor): 150 ms - 500 ms (Heavy setup)",
                "- KURUNG ENGINE COLD START    : 0.78 ms (787 us) [FASTEST]",
                "- KURUNG WARM CACHED RUN      : 0.035 ms (35 us) (28,500 ops/sec)"
            ],
            "box2_title": "READY FOR YOUR AGENT STACK",
            "box2_color": COLOR_GREEN,
            "box2_lines": [
                "- Works with Claude Code, Antigravity CLI, Cursor, and OpenClaw.",
                "- Connects via standard Model Context Protocol (MCP) in 1 line.",
                "- Open-Source dual licensed (MIT OR Apache-2.0).",
                "- GitHub: https://github.com/koodoxz/kurung"
            ]
        }
    ]

    frame_idx = 0
    for act in acts:
        for f in range(act["frames"]):
            img = Image.new("RGB", (WIDTH, HEIGHT), BG_COLOR)
            draw = ImageDraw.Draw(img)
            draw_canvas(draw, act["badge"], act["title"])

            # Left Box (The Threat / Problem)
            draw.rectangle([40, 150, 480, HEIGHT - 40], fill=(15, 23, 42), outline=BORDER_COLOR, width=1)
            draw.rectangle([40, 150, 480, 190], fill=(24, 31, 47), outline=BORDER_COLOR, width=1)
            draw.text((55, 162), act["box1_title"], fill=act["box1_color"], font=FONT_BOLD)

            y1 = 210
            for line in act["box1_lines"]:
                col = TEXT_WHITE if not line.startswith("-") else TEXT_MUTED
                if "[WITHOUT" in line:
                    col = COLOR_RED
                draw.text((55, y1), line, fill=col, font=FONT_TEXT)
                y1 += 26

            # Right Box (The Kurung Defense)
            draw.rectangle([520, 150, WIDTH - 40, HEIGHT - 40], fill=(15, 23, 42), outline=BORDER_COLOR, width=1)
            draw.rectangle([520, 150, WIDTH - 40, 190], fill=(24, 31, 47), outline=BORDER_COLOR, width=1)
            draw.text((535, 162), act["box2_title"], fill=act["box2_color"], font=FONT_BOLD)

            y2 = 210
            for line in act["box2_lines"]:
                col = TEXT_WHITE
                if "[SECURE]" in line or "[RESULT]" in line or "[STATUS]" in line or "[WITH" in line:
                    col = COLOR_GREEN
                elif "[FUEL" in line:
                    col = COLOR_YELLOW
                elif line.startswith("-"):
                    col = TEXT_MUTED
                draw.text((535, y2), line, fill=col, font=FONT_TEXT)
                y2 += 26

            frame_path = FRAMES_DIR / f"frame_{frame_idx:04d}.png"
            img.save(frame_path)
            frame_idx += 1

    print(f"[*] Generated {frame_idx} clear storyboard frames.")
    return frame_idx

def render_story_video():
    total_frames = create_storyboard()
    ffmpeg_bin = Path("C:/Pato/LLM-VIdeo-Transcription/ffmpeg/bin/ffmpeg.exe")
    
    mp4_out = OUTPUT_DIR / "kurung_simulation.mp4"
    gif_out = OUTPUT_DIR / "kurung_simulation.gif"
    
    # 10 fps -> 340 frames = 34 seconds of crystal-clear explanatory walkthrough
    cmd_mp4 = [
        str(ffmpeg_bin), "-y",
        "-framerate", "10",
        "-i", str(FRAMES_DIR / "frame_%04d.png"),
        "-c:v", "libx264",
        "-pix_fmt", "yuv420p",
        "-crf", "18",
        str(mp4_out)
    ]
    subprocess.run(cmd_mp4, check=True)
    print(f"[+] Rendered Clear Story MP4: {mp4_out}")

    cmd_gif = [
        str(ffmpeg_bin), "-y",
        "-framerate", "10",
        "-i", str(FRAMES_DIR / "frame_%04d.png"),
        "-vf", "scale=800:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
        str(gif_out)
    ]
    subprocess.run(cmd_gif, check=True)
    print(f"[+] Rendered Clear Story GIF: {gif_out}")

if __name__ == "__main__":
    render_story_video()
