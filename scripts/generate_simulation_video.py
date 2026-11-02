import os
import subprocess
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

OUTPUT_DIR = Path("C:/Pato/Nusantara-Rust-Suite/kurung/docs/assets")
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
FRAMES_DIR = OUTPUT_DIR / "frames"
FRAMES_DIR.mkdir(parents=True, exist_ok=True)

WIDTH, HEIGHT = 1000, 600
BG_COLOR = (13, 17, 23)
PANEL_BG = (22, 27, 34)
BORDER_COLOR = (48, 54, 61)
TEXT_COLOR = (240, 246, 252)
MUTED_COLOR = (139, 148, 158)
ACCENT_BLUE = (56, 189, 248)
SUCCESS_GREEN = (46, 160, 67)
DANGER_RED = (248, 81, 73)

try:
    FONT_TITLE = ImageFont.truetype("consola.ttf", 22)
    FONT_BODY = ImageFont.truetype("consola.ttf", 15)
    FONT_BOLD = ImageFont.truetype("consolab.ttf", 16)
    FONT_SMALL = ImageFont.truetype("consola.ttf", 13)
except Exception:
    FONT_TITLE = ImageFont.load_default()
    FONT_BODY = ImageFont.load_default()
    FONT_BOLD = ImageFont.load_default()
    FONT_SMALL = ImageFont.load_default()

def draw_window_frame(draw, title):
    # Header bar
    draw.rectangle([0, 0, WIDTH, HEIGHT], fill=BG_COLOR)
    draw.rectangle([20, 20, WIDTH - 20, HEIGHT - 20], fill=PANEL_BG, outline=BORDER_COLOR, width=2)
    
    # Title bar
    draw.rectangle([20, 20, WIDTH - 20, 65], fill=(33, 38, 45), outline=BORDER_COLOR, width=1)
    # Window dots
    draw.ellipse([35, 38, 47, 50], fill=(255, 95, 86))
    draw.ellipse([55, 38, 67, 50], fill=(255, 189, 46))
    draw.ellipse([75, 38, 87, 50], fill=(39, 201, 63))
    
    draw.text((105, 35), title, fill=TEXT_COLOR, font=FONT_BOLD)

def create_frames():
    scenes = [
        # Scene 1: Terminal prompt intro
        {"stage": "intro", "frames": 25, "text": [
            "PS C:\\MECOB> agy --print \"Execute untrusted agent code in kurung sandbox\"",
            "",
            "[*] Host: Windows x86_64 | Policy: #![forbid(unsafe_code)]",
            "[*] Loading Kurung Deterministic Engine (wasmi 0.40 pure-Rust)...",
            "[+] Initialized in 787 microseconds."
        ]},
        # Scene 2: Attack 1 - Infinite Loop
        {"stage": "attack_loop", "frames": 35, "text": [
            "[!] TEST 1: INCOMING ADVERSARIAL THREAT -> Infinite Loop Attack",
            "    Synthesized Code: (loop $inf (br $inf))",
            "    Target Vulnerability: 100% Host CPU Core Lockup",
            "",
            "[>>>] Executing inside Kurung Sandbox...",
            "      CPU Fuel Limit: 50,000 instructions",
            "      Instruction Counter: 10,000... 30,000... 50,000 -> HIT CEILING!",
            "",
            "[SECURE] TRAP: OutOfFuel at exact instruction boundary #50,000",
            "[RESULT] Status: FUEL_EXHAUSTED | Latency: 412 us | Host CPU: 0% SAFE"
        ]},
        # Scene 3: Attack 2 - Memory Bomb
        {"stage": "attack_mem", "frames": 35, "text": [
            "[!] TEST 2: INCOMING ADVERSARIAL THREAT -> Linear Memory Bomb",
            "    Synthesized Code: (memory.grow 10,000 pages) -> 655 MB RAM Allocation",
            "    Target Vulnerability: Host Out-of-Memory (OOM) Crash",
            "",
            "[>>>] Executing inside Kurung Sandbox...",
            "      Configured Memory Ceiling: 16 MiB",
            "      HostContext::memory_growing() intercepted allocation request.",
            "",
            "[SECURE] ResourceLimiter rejected allocation -> Returned -1 to guest",
            "[RESULT] Status: MEMORY_REJECTED | Host Peak RAM: 64 KiB | Host: SAFE"
        ]},
        # Scene 4: Attack 3 - Path Traversal
        {"stage": "attack_vfs", "frames": 35, "text": [
            "[!] TEST 3: INCOMING ADVERSARIAL THREAT -> Filesystem Escape (VFS)",
            "    Synthesized Code: (vfs_read \"../../Windows/System32\")",
            "    Target Vulnerability: Private Host Secret Exfiltration",
            "",
            "[>>>] Executing inside Kurung Sandbox...",
            "      Sanitizer: PathTraversalDetected(\"../../Windows/System32\")",
            "      Virtual Filesystem: Default-Deny In-Memory BTreeMap",
            "",
            "[SECURE] Host access DENIED (-2) | Zero Host Disk Leaks",
            "[RESULT] Status: ACCESS_DENIED | Sandbox: ISOLATED"
        ]},
        # Scene 5: Valid Execution & Cryptographic Receipt
        {"stage": "valid_receipt", "frames": 40, "text": [
            "[+] TEST 4: VALID AGENT COMPUTE -> Sum of Cubes (1^3 + ... + 10^3)",
            "    Return Value: 3025 | Stdout: \"ANTIGRAVITY_AGENT_OK\"",
            "",
            "=== CRYPTOGRAPHIC EXECUTION RECEIPT ===",
            "Module SHA-256 : c53fa1151bb41d12b0827bb39f79afae610709128387c994f3afb8178dabb259",
            "Fuel Consumed  : 64 / 500,000 instructions",
            "Peak RAM       : 65,536 bytes (1 page)",
            "Duration       : 321 us (0.32 ms)",
            "Status         : OK (Deterministic Proof Verified)",
            "",
            "[MECOB] KURUNG CONFINEMENT VERIFICATION: 100% PASSED"
        ]}
    ]

    frame_idx = 0
    for sc in scenes:
        lines = sc["text"]
        for f in range(sc["frames"]):
            img = Image.new("RGB", (WIDTH, HEIGHT), BG_COLOR)
            draw = ImageDraw.Draw(img)
            draw_window_frame(draw, "kurung-sandbox — MECOB Security Confinement Terminal")
            
            y = 85
            # Progressive line printing
            lines_to_show = min(len(lines), int((f / max(1, sc["frames"] - 5)) * len(lines)) + 1)
            for i in range(lines_to_show):
                line = lines[i]
                color = TEXT_COLOR
                if "[SECURE]" in line or "[RESULT]" in line or "[+]" in line or "PASSED" in line:
                    color = SUCCESS_GREEN
                elif "[!]" in line or "TRAP" in line or "DENIED" in line or "HIT" in line:
                    color = DANGER_RED
                elif "[>>>]" in line or "===" in line or "Module" in line:
                    color = ACCENT_BLUE
                elif "[*]" in line:
                    color = MUTED_COLOR

                draw.text((45, y), line, fill=color, font=FONT_BODY)
                y += 24

            frame_path = FRAMES_DIR / f"frame_{frame_idx:04d}.png"
            img.save(frame_path)
            frame_idx += 1

    print(f"[*] Generated {frame_idx} video frames.")
    return frame_idx

def render_mp4_and_gif():
    total_frames = create_frames()
    ffmpeg_bin = Path("C:/Pato/LLM-VIdeo-Transcription/ffmpeg/bin/ffmpeg.exe")
    
    mp4_out = OUTPUT_DIR / "kurung_simulation.mp4"
    gif_out = OUTPUT_DIR / "kurung_simulation.gif"
    
    # 1. Render MP4
    cmd_mp4 = [
        str(ffmpeg_bin), "-y",
        "-framerate", "12",
        "-i", str(FRAMES_DIR / "frame_%04d.png"),
        "-c:v", "libx264",
        "-pix_fmt", "yuv420p",
        "-crf", "18",
        str(mp4_out)
    ]
    subprocess.run(cmd_mp4, check=True)
    print(f"[+] Rendered MP4: {mp4_out}")

    # 2. Render GIF (optimized for GitHub README)
    cmd_gif = [
        str(ffmpeg_bin), "-y",
        "-framerate", "10",
        "-i", str(FRAMES_DIR / "frame_%04d.png"),
        "-vf", "scale=800:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
        str(gif_out)
    ]
    subprocess.run(cmd_gif, check=True)
    print(f"[+] Rendered GIF: {gif_out}")

if __name__ == "__main__":
    render_mp4_and_gif()

