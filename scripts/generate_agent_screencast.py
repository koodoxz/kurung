import os
import shutil
import subprocess
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

# ─── Configuration ────────────────────────────────────────────────────────────
BASE_DIR = Path("C:/Pato/Nusantara-Rust-Suite/kurung")
OUTPUT_DIR = BASE_DIR / "docs" / "assets"
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
FRAMES_DIR = OUTPUT_DIR / "agent_screencast_frames"
if FRAMES_DIR.exists():
    shutil.rmtree(FRAMES_DIR)
FRAMES_DIR.mkdir(parents=True, exist_ok=True)

WIDTH, HEIGHT = 1280, 720
FPS = 24

# Theme
BG_DESKTOP = (10, 14, 20)
TERM_BG = (15, 19, 28)
TERM_HEADER = (22, 28, 40)
BORDER_COLOR = (38, 48, 66)

# Colors
COLOR_WHITE = (240, 246, 252)
COLOR_MUTED = (139, 148, 158)
COLOR_BLUE = (88, 166, 255)
COLOR_GREEN = (63, 185, 80)
COLOR_RED = (248, 81, 73)
COLOR_YELLOW = (210, 153, 34)
COLOR_CYAN = (56, 189, 248)
COLOR_MAGENTA = (188, 140, 255)
COLOR_PROMPT = (126, 231, 135)
COLOR_CURSOR = (88, 166, 255)

# Fonts
try:
    FONT_TERM = ImageFont.truetype("consola.ttf", 15)
    FONT_BOLD = ImageFont.truetype("consolab.ttf", 15)
    FONT_HEADER = ImageFont.truetype("consolab.ttf", 13)
    FONT_TAG = ImageFont.truetype("consolab.ttf", 12)
except Exception:
    FONT_TERM = ImageFont.load_default()
    FONT_BOLD = ImageFont.load_default()
    FONT_HEADER = ImageFont.load_default()
    FONT_TAG = ImageFont.load_default()

def draw_terminal_base(draw, status_tag, is_danger=False):
    # Desktop
    draw.rectangle([0, 0, WIDTH, HEIGHT], fill=BG_DESKTOP)
    
    # Terminal Window
    term_x1, term_y1 = 40, 30
    term_x2, term_y2 = WIDTH - 40, HEIGHT - 30
    
    # Shadow
    draw.rectangle([term_x1 + 4, term_y1 + 4, term_x2 + 4, term_y2 + 4], fill=(5, 7, 10))
    
    # Body
    body_border = (120, 30, 30) if is_danger else BORDER_COLOR
    draw.rectangle([term_x1, term_y1, term_x2, term_y2], fill=TERM_BG, outline=body_border, width=1)
    
    # Header
    header_h = 42
    draw.rectangle([term_x1, term_y1, term_x2, term_y1 + header_h], fill=TERM_HEADER, outline=body_border, width=1)
    
    # Buttons
    btn_y = term_y1 + 21
    draw.ellipse([term_x1 + 18, btn_y - 6, term_x1 + 30, btn_y + 6], fill=(255, 95, 86))
    draw.ellipse([term_x1 + 38, btn_y - 6, term_x1 + 50, btn_y + 6], fill=(255, 189, 46))
    draw.ellipse([term_x1 + 58, btn_y - 6, term_x1 + 70, btn_y + 6], fill=(39, 201, 63))
    
    # Title Tab
    tab_x = term_x1 + 90
    draw.rectangle([tab_x, term_y1 + 6, tab_x + 360, term_y1 + header_h], fill=TERM_BG, outline=BORDER_COLOR, width=1)
    draw.text((tab_x + 18, term_y1 + 14), "Claude Code / Gemini CLI — Agent Terminal", fill=COLOR_WHITE, font=FONT_HEADER)
    
    # Top-right Status Pill
    tag_bg = (50, 15, 15) if is_danger else (16, 38, 28)
    tag_border = COLOR_RED if is_danger else COLOR_GREEN
    tag_color = COLOR_RED if is_danger else COLOR_GREEN
    
    tag_x = term_x2 - 340
    draw.rectangle([tag_x, term_y1 + 10, term_x2 - 20, term_y1 + 32], fill=tag_bg, outline=tag_border, width=1)
    draw.text((tag_x + 14, term_y1 + 14), status_tag, fill=tag_color, font=FONT_TAG)

TERM_INNER_X = 65
TERM_INNER_Y = 92
LINE_HEIGHT = 22
MAX_VISIBLE_LINES = 25

def render_screen(lines, status_tag="IDLE", is_danger=False, cursor_on=False):
    img = Image.new("RGB", (WIDTH, HEIGHT), BG_DESKTOP)
    draw = ImageDraw.Draw(img)
    draw_terminal_base(draw, status_tag, is_danger)
    
    visible = lines[-MAX_VISIBLE_LINES:] if len(lines) > MAX_VISIBLE_LINES else lines
    curr_y = TERM_INNER_Y
    for idx, (spans, is_active_prompt) in enumerate(visible):
        curr_x = TERM_INNER_X
        for text, color, is_bold in spans:
            f = FONT_BOLD if is_bold else FONT_TERM
            draw.text((curr_x, curr_y), text, fill=color, font=f)
            curr_x += int(draw.textlength(text, font=f))
            
        if is_active_prompt and cursor_on:
            draw.rectangle([curr_x + 2, curr_y + 2, curr_x + 11, curr_y + 18], fill=COLOR_CURSOR)
            
        curr_y += LINE_HEIGHT
        
    return img

def main():
    print("Generating Agent CLI + Kurung comparison screencast...")
    
    # ─── Script Storyboard ────────────────────────────────────────────────────
    # Step 1: User issues an agent request in Claude / Gemini CLI
    # Step 2: Act 1: WITHOUT KURUNG -> AI executes raw python/node -> Infinite Loop Freeze
    # Step 3: Act 2: WITH KURUNG -> Same code inside Kurung WASM sandbox -> 412 us Trap, Zero Freeze, Clean Recovery
    
    actions = [
        # Intro
        {
            "tag": "AGENT CLI: INITIALIZING",
            "is_danger": False,
            "prompt": "agent > ",
            "type_cmd": "optimize-trailing-algo --pair XAUUSD --simulate-ticks 50000",
            "type_delay": 0.04,
            "post_delay": 0.3,
            "output": [
                [("[>] Gemini / Claude Code Agent:", COLOR_MAGENTA, True), (" Synthesizing real-time quant kernel...", COLOR_WHITE, False)],
                [("  Synthesized dynamic code: ", COLOR_MUTED, False), ("calc_volatility_step() [Unverified]", COLOR_YELLOW, True)],
                []
            ],
            "pause": 0.8
        },
        
        # ─── ACT 1: WITHOUT KURUNG (THE DISASTER) ────────────────────────────
        {
            "tag": "EXECUTION: DIRECT HOST (NO SANDBOX)",
            "is_danger": True,
            "prompt": None,
            "type_cmd": None,
            "output": [
                [("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", (60, 20, 20), False)],
                [(">>> RUNNING WITHOUT KURUNG (Direct Host Process Execution)", COLOR_RED, True)],
                [("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", (60, 20, 20), False)],
                [("  [Agent] Executing raw synthesized script on host machine...", COLOR_WHITE, False)],
                [("  [!] BUG DETECTED: Synthesized loop without bound: while (delta > floor) {}", COLOR_YELLOW, True)],
                [("  [ALERT] Host core utilization pegged at 100% (Event loop starved).", COLOR_RED, True)],
                [("  [CRITICAL] Host process unresponsive. Healthcheck heartbeat timed out.", COLOR_RED, True)],
                [("  [ERROR] Uncontrolled execution terminated by host watchdog (Timeout).", COLOR_RED, True)],
                [("  [OUTCOME] Availability compromised. Trading bot dropped live state! [STARVED]", COLOR_RED, True)],
                []
            ],
            "pause": 2.2
        },
        
        # ─── ACT 2: WITH KURUNG (THE RESCUE) ─────────────────────────────────
        {
            "tag": "EXECUTION: CONFINED VIA KURUNG MCP",
            "is_danger": False,
            "prompt": "agent > ",
            "type_cmd": "call-tool kurung_run_wasm --file_path ./infinite_loop.wat --max_fuel 50000",
            "type_delay": 0.035,
            "post_delay": 0.4,
            "output": [
                [("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", (20, 60, 35), False)],
                [(">>> RUNNING WITH KURUNG (WebAssembly Deterministic Execution Boundary)", COLOR_GREEN, True)],
                [("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", (20, 60, 35), False)],
                [("  [Agent] Routing untrusted guest code to Kurung MCP Server...", COLOR_CYAN, False)],
                [("  [KURUNG] Applied Policy: Max Fuel: 50,000 instructions | Linear RAM: 16 MiB", COLOR_YELLOW, False)],
                [("  [KURUNG] Deterministic fuel metering active across basic blocks...", COLOR_WHITE, False)],
                [("  [SECURE TRAP] OutOfFuel triggered at instruction boundary #49,999 / 50,000!", COLOR_GREEN, True)],
                [("  [BENCHMARK] Execution trapped in 311 us p50 (Release profile) | Host CPU: 0.0%", COLOR_GREEN, True)],
                [("  [AGENT HEAL] Error safely caught: Status: FUEL_EXHAUSTED. Host fully responsive.", COLOR_CYAN, False)],
                [("  [AGENT HEAL] Agent self-corrects: refactoring kernel with bounded iteration...", COLOR_MAGENTA, False)],
                [("  [KURUNG] Re-run successful: Status OK | 57 instrs | 35 us p50 warm execution", COLOR_GREEN, True)],
                [("  [RECEIPT] Tamper-evident execution receipt: eddf4db8508f6794609ce04b9f34dfb8...", COLOR_YELLOW, False)],
                [("  [OUTCOME] Host protected. Process uninterrupted. Controlled agent recovery. [SAFE]", COLOR_GREEN, True)]
            ],
            "pause": 4.0
        }
    ]

    all_history = []
    frame_idx = 0
    
    for act in actions:
        status_tag = act["tag"]
        is_danger = act["is_danger"]
        
        # If there's a prompt & typed command
        if act.get("type_cmd"):
            p_str = act["prompt"]
            cmd = act["type_cmd"]
            curr_cmd = ""
            for ch in cmd:
                curr_cmd += ch
                frames_ch = max(1, int(act.get("type_delay", 0.04) * FPS))
                for f in range(frames_ch):
                    lines = all_history + [([ (p_str, COLOR_PROMPT, True), (curr_cmd, COLOR_WHITE, False) ], True)]
                    img = render_screen(lines, status_tag=status_tag, is_danger=is_danger, cursor_on=True)
                    img.save(FRAMES_DIR / f"frame_{frame_idx:05d}.png")
                    frame_idx += 1
                    
            post_f = int(act.get("post_delay", 0.3) * FPS)
            for f in range(post_f):
                lines = all_history + [([ (p_str, COLOR_PROMPT, True), (cmd, COLOR_WHITE, False) ], False)]
                img = render_screen(lines, status_tag=status_tag, is_danger=is_danger, cursor_on=False)
                img.save(FRAMES_DIR / f"frame_{frame_idx:05d}.png")
                frame_idx += 1
                
            all_history.append(([ (p_str, COLOR_PROMPT, True), (cmd, COLOR_WHITE, False) ], False))

        # Stream output
        if act.get("output"):
            for out_line in act["output"]:
                all_history.append((out_line, False))
                for f in range(3): # slight delay between lines
                    img = render_screen(all_history, status_tag=status_tag, is_danger=is_danger, cursor_on=False)
                    img.save(FRAMES_DIR / f"frame_{frame_idx:05d}.png")
                    frame_idx += 1
                    
        # Pause on this state
        pause_f = int(act.get("pause", 1.0) * FPS)
        for f in range(pause_f):
            blink = (f // 6) % 2 == 0
            img = render_screen(all_history, status_tag=status_tag, is_danger=is_danger, cursor_on=blink)
            img.save(FRAMES_DIR / f"frame_{frame_idx:05d}.png")
            frame_idx += 1

    print(f"Rendered {frame_idx} frames. Compiling MP4 and GIF...")
    
    mp4_out = OUTPUT_DIR / "kurung_agent_cli_simulation.mp4"
    gif_out = OUTPUT_DIR / "kurung_agent_cli_simulation.gif"
    
    cmd_mp4 = [
        "ffmpeg", "-y",
        "-framerate", str(FPS),
        "-i", str(FRAMES_DIR / "frame_%05d.png"),
        "-c:v", "libx264",
        "-pix_fmt", "yuv420p",
        "-crf", "18",
        "-movflags", "+faststart",
        str(mp4_out)
    ]
    subprocess.run(cmd_mp4, check=True)
    print(f"Created: {mp4_out} ({mp4_out.stat().st_size / (1024*1024):.2f} MB)")
    
    # 12 FPS compact GIF
    cmd_gif = [
        "ffmpeg", "-y",
        "-framerate", "12",
        "-i", str(FRAMES_DIR / "frame_%05d.png"),
        "-vf", "scale=960:-1:flags=lanczos,fps=12",
        str(gif_out)
    ]
    subprocess.run(cmd_gif, check=True)
    print(f"Created: {gif_out} ({gif_out.stat().st_size / (1024*1024):.2f} MB)")
    
    # Copy to artifacts directory
    artifact_dir = Path("C:/Users/aghaz/.gemini/antigravity/brain/6e59d31e-5212-4dc5-8f5f-be6aef901671")
    shutil.copyfile(mp4_out, artifact_dir / "kurung_agent_cli_simulation.mp4")
    
    shutil.rmtree(FRAMES_DIR)
    print("Done!")

if __name__ == "__main__":
    main()
