# Where PCTwin is

Progress updates for anyone following along. The newest update is at the top. We add one whenever something worth telling has landed.

## Right now

- **Stage:** building the move itself (stage 1 of the plan). The foundations (stage 0) are done.
- **Ready to use?** Not yet. Please don't move real files with it.
- **Next:** the rest of the move screens, and finishing the checks that make sure every copied file is stored correctly.

## 15 October 2026

**Done**

- **Pairing the two laptops works, on screen.** The old laptop shows a short code, and you type it on the new laptop. Both laptops then show a number, and you confirm they match. The old laptop counts as paired only once the new one confirms over the encrypted link. If the connection drops before the move starts, both screens say so. Nothing ever shows "paired" when it isn't.
- **A dropped move carries on.** A move that loses its connection reconnects and continues from where it stopped, without asking for a new code.
- **Files land only where you approved.** The new laptop writes a received file only in the place your plan says, at exactly the size it was told. Anything else is refused.
- **Quick Send (engine).** You can send chosen files to a laptop nearby, outside a full move. The receiving laptop picks where they go, and nothing received is opened or run. The screens for it are still to come.
- **Testing on Windows is reliable again.** A network guard on our Windows test machines was blocking a security check that downloads need. Those test machines now pass.

**In progress**

- **Checking every file after it is written.** Before a copied file gets its name, the new laptop reads it back from the drive and compares it, piece by piece, with what the old laptop sent. An independent review found a case where Windows quietly reads from memory instead of the drive (folders that Windows compresses). Each file is now judged on its own, and the report says plainly when a file could only be checked from memory. That fix is being reviewed again.
- **Sending damaged pieces again.** If part of a file arrives damaged, or reads back wrong, only that part is sent again, at most twice. A file that still does not match is reported, never shown as copied. This is in review.
- **The move screens:** welcome, choosing which laptop is which, the plan to approve, progress, and the final report. They are being built from our designs.

**How we check our work**

- Every change runs our tests on Windows, Mac and Linux.
- Anything touching pairing, encryption or how files are written gets a second, independent review before it goes in.
- We also break the code on purpose, to make sure the tests notice.
