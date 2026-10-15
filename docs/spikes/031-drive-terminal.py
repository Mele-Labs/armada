import os, pty, time, select, re, sys
env = dict(os.environ, ARMADA_DRONE="1")
pid, fd = pty.fork()
if pid == 0:
    os.chdir("/tmp/perm-spike-work")
    os.execvpe("claude", ["claude", "--model", "haiku", "--permission-mode", "default", "--plugin-dir", "/tmp/perm-spike"], env)
out = b""
def pump(secs):
    global out
    end = time.time() + secs
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.2)
        if r:
            try: out += os.read(fd, 65536)
            except OSError: return
def screen():
    return re.sub(rb"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07]*\x07|\x1b.", b"", out).decode("utf-8", "replace")
pump(6)
if "trust" in screen().lower():
    os.write(fd, b"\x1b[B"); pump(1); os.write(fd, b"\r"); pump(5)
os.write(fd, b"Use the Bash tool to run exactly this command and nothing else: touch /tmp/perm-spike-work/touched")
pump(1); os.write(fd, b"\r")
pump(14)
mark = len(screen())
print("=== at 15s ===")
print(screen()[-1500:])
pump(20)
print("=== at 35s ===")
print(screen()[-1800:])
os.write(fd, b"\x03"); pump(1); os.write(fd, b"\x03"); pump(2)
