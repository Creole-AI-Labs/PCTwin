# Where PCTwin is

Progress updates for anyone following along. The newest update is at the top. We add one whenever something worth telling has landed.

## Right now

- **Stage:** building the move itself (stage 1 of the plan). The foundations (stage 0) are done.
- **Ready to use?** Not yet. Please don't move real files with it.
- **Next:** saying everything on the screens in plain words, connecting the screens to the move itself, and finishing safe copying onto USB sticks and memory cards (FAT and exFAT drives).

## 21 October 2026

**In progress**

- **Speed and time left, measured, never guessed.** While a move runs, PCTwin works out its speed from what it really moved in about the last 20 seconds, and the time left from that. In the first few seconds, or when it can't know, it says so instead of guessing. When nothing is moving, it says it is waiting rather than showing a time that keeps growing. This is ready for review; the screens will show it once they are connected to the move.
- **Undo on a Mac puts copies in the Trash.** Until now, undo on a Mac kept every copy, because a Mac can't prove that no other app is still writing to a file. Now undo moves each copy PCTwin made, and that you haven't changed, into the Trash on the same drive, so nothing is ever deleted and you can put anything back. Before it moves a file, PCTwin first sets the copy aside under a private name and checks that it is the very file it made, still unchanged; if another app swapped in a file of its own, that file goes straight back where it was and never reaches the Trash. A file you edited after the move is always kept. On USB sticks and memory cards (FAT and exFAT drives), undo keeps every copy, because those drives reuse the numbers that tell files apart, and the undo screen will say so before you start. Its tests pass on Mac test machines; it is waiting for an independent review.

## 16 October 2026

**Done**

- **Every copied file is read back from the drive and checked** before it gets its name. If Windows could only check a file from memory (for example in a folder Windows compresses), the report says so plainly.
- **Damaged pieces are sent again.** If part of a file arrives damaged, or reads back wrong, only that part is sent again, at most twice. A file that still does not match is reported, never shown as copied.
- **The old laptop is protected from being worn out.** A faulty or misbehaving new laptop can't make the old laptop read the same files over and over. After a dropped connection, what is sent again is limited to what the last connection actually achieved, and each file can be read only a few times its size. A poor connection makes a move slower, never a failure.
- **The move screens are built:** welcome, choosing which laptop is which, who's moving, what to do first, the scan and the plan to approve, move progress with pause and stop, the final report, and Quick Send with its history. They follow our designs, and nothing on them is for sale. They are not yet connected to the move itself.
- **Our tests judge by progress, not the clock.** A slow test machine no longer fails a test; a test that is truly stuck still does.

- **The screens now speak all seven launch languages.** French, Spanish, Portuguese, Arabic, Chinese and Hindi were made ahead of time by machine translation, in the same plain words as the English. Arabic reads right to left. Automatic checks stop a broken translation (a missing number, a wrong plural, a changed product name) from shipping. A second tool translates the text back to English to look for changed meaning; that check has finished for Arabic and part of Hindi, and the rest is waiting on a free daily limit.

**In progress**

- **USB sticks and memory cards.** These drives (FAT and exFAT) can't always give a file its name safely in one step. On those drives PCTwin writes the file hidden under its real name, checks it, then shows it, and a copy cut off halfway is cleaned up or finished on the next start. An independent review found problems, which are being fixed and reviewed again.
- **Plain words on every screen.** Some screen text still uses technical words from our designs. It is being rewritten so anyone can follow it.
- **Connecting the screens to the move, safely.** The screens are treated as untrusted: they can ask the app for a short, fixed list of things (window size, tray words, pairing), and the app refuses anything else, such as opening files, windows or other programs. Everything the app sends back is checked against the app's own examples before a screen shows it, and automatic tests fail if the two ever stop matching. When a file could only be checked from memory, the report now says why, in plain words, for each of the five reasons. A second independent review found ways round the first checks; the checks now try every request the app knows and fail if anything beyond the list gets through, and they are being reviewed again.

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
