# Bad-network simulation and test machines

PCTwin moves over the home or office Wi-Fi people already have. These tools make a test network
slow, jittery or lossy on purpose, so a move is proven to pause, resume and finish honestly on a
bad connection, not only on a fast one.

| System | Tool | How |
|---|---|---|
| Linux | netem (in the kernel) | `sudo fixtures/netsim/linux-netem.sh on lo 20ms 5ms 1%`, then `off`. CI runs a whole two-account move through it. |
| macOS | dummynet (`dnctl` and `pfctl`) | `sudo fixtures/netsim/macos-dummynet.sh on 40 0.01`, then `off`. Shapes traffic to `127.0.0.3`. |
| macOS | Network Link Conditioner | A settings pane in Apple's "Additional Tools for Xcode" download. Pick a profile ("3G", "Very Bad Network") or make one. Not scriptable, so used by hand on owned Macs. |
| Windows | [Clumsy](https://github.com/jagt/clumsy) | Free, open source, runs as administrator, built on WinDivert. Set a filter first (for example `tcp and (tcp.DstPort == 51000 or tcp.SrcPort == 51000)` for one move's port), then tick Lag, Drop or Throttle. Its numbers are approximate, which is fine: the test is that the move ends honestly, not a speed figure. |

What to check on a bad network: the move finishes or pauses with a clear reason; nothing is
shown as copied that did not arrive whole; a dropped connection resumes without starting over;
the old laptop is unchanged (`pctwin-lab snapshot` before and after, then `compare`).

## Local virtual machines

For live pairing, permissions and power tests that hosted CI runners cannot do. All free:

- **Windows 11:** Microsoft's evaluation edition (90 days) as an ISO, or the "Windows 11
  development environment" virtual machine image, in Hyper-V (Windows Pro) or VirtualBox. Make
  two machines on one virtual switch so they see each other as on one network; a NAT-only network
  hides them from each other.
- **Ubuntu:** the desktop ISO in VirtualBox, Hyper-V or GNOME Boxes. Use a bridged or shared
  internal network for both machines.
- **Fedora:** Fedora Workstation ISO the same way; it uses Wayland and SELinux by default, which
  the Ubuntu machine does not, so test both.
- **macOS:** only on Apple hardware under Apple's licence (for example UTM on an owned Mac). CI
  runners prove builds and file behaviour, not the permission prompts people see.

Snapshot each machine before a test move and roll back after, so every run starts clean. Never
put real personal files on a test machine: use `pctwin-lab make` to fill it.
