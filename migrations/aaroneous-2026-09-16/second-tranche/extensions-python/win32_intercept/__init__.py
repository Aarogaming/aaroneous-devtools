#!/usr/bin/env python3
"""
Win32 Intercept Perimeter - Spatial-Kinetic Screen Capture & HID Bridge

Captures any workspace screen as a raw 128x128 float grid and converts
intent vectors back to SendInput hardware events.

This module treats the entire Windows desktop as a game interface:
- Screen capture → normalized 128x128 spatial matrix
- Motor intent → SendInput mouse/keyboard events
- Epigenetic gate mask → selective region capture
"""

import ctypes
import ctypes.wintypes as wintypes
import numpy as np
import time
import mmap
import os
import struct
from typing import Optional, Tuple
from dataclasses import dataclass

# Win32 Constants
CAPTUREBLT = 0x40000000
SRCCOPY = 0x00CC0020
DIB_RGB_COLORS = 0

# Grid dimensions
GRID_WIDTH = 128
GRID_HEIGHT = 128
GRID_SIZE = GRID_WIDTH * GRID_HEIGHT

# Synapse shared memory layout
SYNAPSE_MAGIC = b'AAS1'
SYNAPSE_SIZE = 1024 * 1024  # 1MB shared memory


@dataclass
class MotorIntent:
    """Motor intent output from the reflex kernel."""
    delta_x: float
    delta_y: float
    binary_action_register: int  # 64-bit action flags


# Action register bit flags
ACTION_MOUSE_MOVE = 1 << 0
ACTION_MOUSE_LEFT_DOWN = 1 << 1
ACTION_MOUSE_LEFT_UP = 1 << 2
ACTION_MOUSE_RIGHT_DOWN = 1 << 3
ACTION_MOUSE_RIGHT_UP = 1 << 4
ACTION_MOUSE_WHEEL = 1 << 5
ACTION_KEY_PRESS = 1 << 6
ACTION_KEY_RELEASE = 1 << 7
ACTION_CLICK = 1 << 8
ACTION_DOUBLE_CLICK = 1 << 9
ACTION_DRAG_START = 1 << 10
ACTION_DRAG_END = 1 << 11


class Win32ScreenCapture:
    """Captures the Windows desktop as a normalized 128x128 float grid."""

    def __init__(self, monitor_index: int = 0):
        self.monitor_index = monitor_index
        self._hdc_screen = None
        self._hdc_memory = None
        self._hbitmap = None
        self._bitmap_data = None
        self._initialized = False

    def initialize(self):
        """Initialize GDI capture resources."""
        user32 = ctypes.windll.user32
        gdi32 = ctypes.windll.gdi32

        # Get screen DC
        self._hdc_screen = user32.GetDC(0)
        if not self._hdc_screen:
            raise RuntimeError("Failed to get screen DC")

        # Create compatible DC
        self._hdc_memory = gdi32.CreateCompatibleDC(self._hdc_screen)
        if not self._hdc_memory:
            raise RuntimeError("Failed to create memory DC")

        # Get screen dimensions
        self._screen_width = user32.GetSystemMetrics(0)
        self._screen_height = user32.GetSystemMetrics(1)

        # Create compatible bitmap
        self._hbitmap = gdi32.CreateCompatibleBitmap(
            self._hdc_screen, GRID_WIDTH, GRID_HEIGHT
        )
        if not self._hbitmap:
            raise RuntimeError("Failed to create bitmap")

        # Select bitmap into memory DC
        gdi32.SelectObject(self._hdc_memory, self._hbitmap)

        # Allocate buffer for bitmap data (32-bit RGBA)
        self._bitmap_data = ctypes.create_string_buffer(GRID_WIDTH * GRID_HEIGHT * 4)

        self._initialized = True

    def capture_frame(self, gate_mask: Optional[np.ndarray] = None) -> np.ndarray:
        """
        Capture screen and return as 128x128 normalized float grid.

        Args:
            gate_mask: Optional 256-element boolean array indicating active sectors.
                      If None, captures full screen.

        Returns:
            Normalized float array [0, 1] of shape (128, 128)
        """
        if not self._initialized:
            self.initialize()

        user32 = ctypes.windll.user32
        gdi32 = ctypes.windll.gdi32

        # Blit screen to memory DC (stretched to 128x128)
        result = gdi32.StretchBlt(
            self._hdc_memory, 0, 0, GRID_WIDTH, GRID_HEIGHT,
            self._hdc_screen, 0, 0, self._screen_width, self._screen_height,
            SRCCOPY | CAPTUREBLT
        )
        if not result:
            raise RuntimeError("StretchBlt failed")

        # Get bitmap bits
        class BITMAPINFOHEADER(ctypes.Structure):
            _fields_ = [
                ("biSize", wintypes.DWORD),
                ("biWidth", wintypes.LONG),
                ("biHeight", wintypes.LONG),
                ("biPlanes", wintypes.WORD),
                ("biBitCount", wintypes.WORD),
                ("biCompression", wintypes.DWORD),
                ("biSizeImage", wintypes.DWORD),
                ("biXPelsPerMeter", wintypes.LONG),
                ("biYPelsPerMeter", wintypes.LONG),
                ("biClrUsed", wintypes.DWORD),
                ("biClrImportant", wintypes.DWORD),
            ]

        bmi = BITMAPINFOHEADER()
        bmi.biSize = ctypes.sizeof(BITMAPINFOHEADER)
        bmi.biWidth = GRID_WIDTH
        bmi.biHeight = -GRID_HEIGHT  # Top-down
        bmi.biPlanes = 1
        bmi.biBitCount = 32
        bmi.biCompression = 0

        gdi32.GetDIBits(
            self._hdc_memory, self._hbitmap,
            0, GRID_HEIGHT,
            self._bitmap_data,
            ctypes.byref(bmi),
            DIB_RGB_COLORS
        )

        # Convert to numpy array
        raw = np.frombuffer(self._bitmap_data, dtype=np.uint8)
        rgba = raw.reshape((GRID_HEIGHT, GRID_WIDTH, 4))

        # Convert to grayscale luminance
        gray = (
            rgba[:, :, 0].astype(np.float32) * 0.299 +
            rgba[:, :, 1].astype(np.float32) * 0.587 +
            rgba[:, :, 2].astype(np.float32) * 0.114
        )

        # Normalize to [0, 1]
        frame = gray / 255.0

        # Apply epigenetic gate mask if provided
        if gate_mask is not None:
            frame = self._apply_gate_mask(frame, gate_mask)

        return frame

    def _apply_gate_mask(self, frame: np.ndarray, gate_mask: np.ndarray) -> np.ndarray:
        """Zero out inactive sectors based on epigenetic gate mask."""
        SECTOR_SIZE = 8
        SECTORS_PER_ROW = 16

        for sector_idx, active in enumerate(gate_mask):
            if not active:
                sector_y = sector_idx // SECTORS_PER_ROW
                sector_x = sector_idx % SECTORS_PER_ROW
                y_start = sector_y * SECTOR_SIZE
                x_start = sector_x * SECTOR_SIZE
                frame[y_start:y_start+SECTOR_SIZE, x_start:x_start+SECTOR_SIZE] = 0.0

        return frame

    def cleanup(self):
        """Release GDI resources."""
        if self._initialized:
            gdi32 = ctypes.windll.gdi32
            user32 = ctypes.windll.user32

            if self._hbitmap:
                gdi32.DeleteObject(self._hbitmap)
            if self._hdc_memory:
                gdi32.DeleteDC(self._hdc_memory)
            if self._hdc_screen:
                user32.ReleaseDC(0, self._hdc_screen)

            self._initialized = False

    def __del__(self):
        self.cleanup()


class HIDOutputBridge:
    """Converts motor intents to Win32 SendInput hardware events."""

    def __init__(self):
        self._user32 = ctypes.windll.user32
        self._mouse_sensitivity = 1.0
        self._key_state = {}

    def execute_intent(self, intent: MotorIntent):
        """Execute a motor intent as hardware events."""
        actions = intent.binary_action_register

        if actions & ACTION_MOUSE_MOVE:
            self._move_mouse(intent.delta_x, intent.delta_y)

        if actions & ACTION_MOUSE_LEFT_DOWN:
            self._mouse_down(0x0002)  # MOUSEEVENTF_LEFTDOWN

        if actions & ACTION_MOUSE_LEFT_UP:
            self._mouse_up(0x0004)  # MOUSEEVENTF_LEFTUP

        if actions & ACTION_MOUSE_RIGHT_DOWN:
            self._mouse_down(0x0008)  # MOUSEEVENTF_RIGHTDOWN

        if actions & ACTION_MOUSE_RIGHT_UP:
            self._mouse_up(0x0010)  # MOUSEEVENTF_RIGHTUP

        if actions & ACTION_MOUSE_WHEEL:
            self._mouse_wheel(int(intent.delta_y * 120))

        if actions & ACTION_CLICK:
            self._click()

        if actions & ACTION_DOUBLE_CLICK:
            self._double_click()

        if actions & ACTION_DRAG_START:
            self._mouse_down(0x0002)

        if actions & ACTION_DRAG_END:
            self._mouse_up(0x0004)

    def _move_mouse(self, delta_x: float, delta_y: float):
        """Move mouse by relative delta."""
        dx = int(delta_x * self._mouse_sensitivity)
        dy = int(delta_y * self._mouse_sensitivity)

        if dx == 0 and dy == 0:
            return

        class MOUSEINPUT(ctypes.Structure):
            _fields_ = [
                ("dx", ctypes.c_long),
                ("dy", ctypes.c_long),
                ("mouseData", ctypes.c_ulong),
                ("dwFlags", ctypes.c_ulong),
                ("time", ctypes.c_ulong),
                ("dwExtraInfo", ctypes.POINTER(ctypes.c_ulong)),
            ]

        class INPUT(ctypes.Structure):
            _fields_ = [
                ("type", ctypes.c_ulong),
                ("mi", MOUSEINPUT),
            ]

        mi = MOUSEINPUT()
        mi.dx = dx
        mi.dy = dy
        mi.dwFlags = 0x0001  # MOUSEEVENTF_MOVE

        inp = INPUT()
        inp.type = 0  # INPUT_MOUSE
        inp.mi = mi

        self._user32.SendInput(1, ctypes.byref(inp), ctypes.sizeof(inp))

    def _mouse_down(self, flag: int):
        class MOUSEINPUT(ctypes.Structure):
            _fields_ = [
                ("dx", ctypes.c_long),
                ("dy", ctypes.c_long),
                ("mouseData", ctypes.c_ulong),
                ("dwFlags", ctypes.c_ulong),
                ("time", ctypes.c_ulong),
                ("dwExtraInfo", ctypes.POINTER(ctypes.c_ulong)),
            ]

        class INPUT(ctypes.Structure):
            _fields_ = [
                ("type", ctypes.c_ulong),
                ("mi", MOUSEINPUT),
            ]

        mi = MOUSEINPUT()
        mi.dwFlags = flag

        inp = INPUT()
        inp.type = 0
        inp.mi = mi

        self._user32.SendInput(1, ctypes.byref(inp), ctypes.sizeof(inp))

    def _mouse_up(self, flag: int):
        self._mouse_down(flag)

    def _mouse_wheel(self, delta: int):
        class MOUSEINPUT(ctypes.Structure):
            _fields_ = [
                ("dx", ctypes.c_long),
                ("dy", ctypes.c_long),
                ("mouseData", ctypes.c_ulong),
                ("dwFlags", ctypes.c_ulong),
                ("time", ctypes.c_ulong),
                ("dwExtraInfo", ctypes.POINTER(ctypes.c_ulong)),
            ]

        class INPUT(ctypes.Structure):
            _fields_ = [
                ("type", ctypes.c_ulong),
                ("mi", MOUSEINPUT),
            ]

        mi = MOUSEINPUT()
        mi.mouseData = delta
        mi.dwFlags = 0x0800  # MOUSEEVENTF_WHEEL

        inp = INPUT()
        inp.type = 0
        inp.mi = mi

        self._user32.SendInput(1, ctypes.byref(inp), ctypes.sizeof(inp))

    def _click(self):
        self._mouse_down(0x0002)
        time.sleep(0.01)
        self._mouse_up(0x0004)

    def _double_click(self):
        self._click()
        time.sleep(0.05)
        self._click()


class SynapseWriter:
    """Writes captured frames to shared memory for the reflex kernel."""

    def __init__(self, name: str = "SAB_STORE"):
        self.name = name
        self.path = os.path.join(
            os.environ["LOCALAPPDATA"], "Temp", f"{name}.synapse"
        )
        self._mm = None
        self._f = None

    def open(self, size: int = SYNAPSE_SIZE):
        """Open or create shared memory region."""
        self._f = open(self.path, "r+b")
        self._mm = mmap.mmap(self._f.fileno(), size)

    def write_frame(self, frame: np.ndarray, frame_id: int = 0):
        """Write a 128x128 float frame to shared memory."""
        if self._mm is None:
            self.open()

        # Write magic + frame ID
        self._mm.seek(0)
        self._mm.write(SYNAPSE_MAGIC)
        self._mm.write(struct.pack('<Q', frame_id))

        # Write frame data as flattened float32
        flat = frame.flatten().astype(np.float32)
        self._mm.seek(12)  # After magic (4) + frame_id (8)
        self._mm.write(flat.tobytes())

    def close(self):
        if self._mm:
            self._mm.close()
        if self._f:
            self._f.close()


def run_intercept_loop(
    capture_fps: float = 30.0,
    output_synapse: str = "SAB_STORE",
    enable_hid: bool = False,
):
    """
    Main intercept loop: capture → write to synapse → (optional) execute HID.

    This runs as a standalone process that feeds the spatial-kinetic engine.
    """
    print(f"[Win32 Intercept] Starting at {capture_fps} FPS")
    print(f"[Win32 Intercept] Output: {output_synapse}")
    print(f"[Win32 Intercept] HID Bridge: {'enabled' if enable_hid else 'disabled'}")

    capture = Win32ScreenCapture()
    capture.initialize()

    writer = SynapseWriter(output_synapse)
    writer.open()

    hid_bridge = HIDOutputBridge() if enable_hid else None

    frame_id = 0
    frame_interval = 1.0 / capture_fps

    try:
        while True:
            loop_start = time.time()

            # Capture frame
            frame = capture.capture_frame()

            # Write to synapse
            writer.write_frame(frame, frame_id)

            # (Optional) Read motor intent from synapse and execute
            if hid_bridge:
                intent = _read_motor_intent(writer._mm)
                if intent:
                    hid_bridge.execute_intent(intent)

            frame_id += 1

            # Frame rate limiting
            elapsed = time.time() - loop_start
            sleep_time = frame_interval - elapsed
            if sleep_time > 0:
                time.sleep(sleep_time)

            if frame_id % 60 == 0:
                fps = 60.0 / (time.time() - loop_start + frame_interval * 59)
                print(f"[Win32 Intercept] Frame {frame_id} | {fps:.1f} FPS")

    except KeyboardInterrupt:
        print("\n[Win32 Intercept] Shutting down...")
    finally:
        capture.cleanup()
        writer.close()


def _read_motor_intent(mm: mmap.mmap) -> Optional[MotorIntent]:
    """Read motor intent from shared memory (written by reflex kernel)."""
    try:
        # Intent is written at offset 12 + 128*128*4 = 65548
        intent_offset = 12 + GRID_SIZE * 4
        mm.seek(intent_offset)
        data = mm.read(16)  # 2x float32 + 1x uint64
        if len(data) < 16:
            return None

        delta_x, delta_y = struct.unpack('<ff', data[:8])
        action_register = struct.unpack('<Q', data[8:16])[0]

        return MotorIntent(delta_x, delta_y, action_register)
    except Exception:
        return None


if __name__ == '__main__':
    import argparse

    parser = argparse.ArgumentParser(description='Win32 Intercept Perimeter')
    parser.add_argument('--fps', type=float, default=30.0, help='Capture FPS')
    parser.add_argument('--synapse', type=str, default='SAB_STORE', help='Synapse name')
    parser.add_argument('--hid', action='store_true', help='Enable HID output bridge')

    args = parser.parse_args()
    run_intercept_loop(args.fps, args.synapse, args.hid)
