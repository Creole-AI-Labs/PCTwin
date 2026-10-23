# Where PCTwin is

Progress updates for anyone following along. The newest update is at the top. We add one whenever something worth telling has landed.

## Right now

- **Stage:** building the move itself (stage 1 of the plan). The foundations (stage 0) are done.
- **Ready to use?** Not yet. Please don't move real files with it.
- **Next:** the last pieces of this stage, independent reviews of the newest parts, then one full test run before it is merged.

## 24 October 2026

**In progress (the full tests now run on Windows, Mac and Linux)**

- **The engine's automatic tests pass on six test machines:** Windows (Intel and Arm), Mac (Apple and Intel chips) and Linux (Intel and Arm); the full test of moving everyone with real accounts is being finished. Running everything for the first time found real problems that a single laptop never could, and each is fixed: on Mac and Linux, moving other people could never have started (a folder the helper rightly refused, and a folder a Mac doesn't have); on Windows, a new account's home folder was never made; your own Desktop, Documents and Pictures were missed in a move of everyone; and on Windows, PowerShell 7 could hide who owns each folder.
- **Moving other people now works on a Mac too.** Each move asks an administrator in the Mac's own password window, naming whose files will be read; PCTwin never sees the password. The first time, the Mac asks once for PCTwin to be allowed in System Settings.
- **The administrator helper is harder to trick.** On Windows it finds system folders by asking Windows itself, never trusting settings a program could change; it reads another person's file only through the folders it found it in, so a folder swapped for a shortcut leads nowhere; and it runs with administrator rights only from inside Program Files. The Windows installer now installs for everyone on the laptop, asking an administrator once.
- **The installers carry everything.** The helper programs now ship with the app on all three systems, checked automatically in every test run.
- **A restart never loses a move.** While a move is under way, PCTwin opens by itself when you sign in again, ready to pick up.
- **If both laptops chose the same role,** PCTwin says so and lets you switch one.
- **Before the move,** a short checklist (plugged in, lids open, near the router); **during it,** one helpful tip at a time under the progress bar; **after it,** a check of photos, videos, music and documents planned against what arrived, and each line of the report says if a file changed since the scan or is no longer there.
- **A failing drive stops at once.** When a drive keeps failing, the move pauses and nothing more is written there, even files already on their way; they wait to be sent elsewhere.
- **"Start a full move"** is the main button's new name.

## 22 October 2026

**In progress (the app now does a whole move)**

- **The screens are connected to the move.** Pair the laptops, and the old one finds your files while the new one waits; the new laptop shows the plan (whether it fits, about how long, anything that won't go as it is, and where each folder lands), you can untick a folder to leave it behind, and nothing moves until you approve. Then progress with pause and stop, and at the end what happened to every file.
- **Clear pages when something goes wrong.** A lost connection, a plan that wasn't approved, a drive that isn't keeping files, a move that couldn't start: each page says what happened, what is kept, that your old laptop wasn't changed, and what to do next. The technical words are only under "More details".
- **Get help with a short code.** Any problem page can make a 12-character code to send by WhatsApp or any app. It holds only the kind of problem, how far the move got, the PCTwin version and the system, never names, files or passwords.
- **Picks up safely after a restart.** If a laptop restarts or loses power mid-move, PCTwin opens with "Pick up safely", tidies up, and the next pairing carries on without copying anything twice.
- **Moving other people, in the app (Windows and Linux).** On the old laptop you tick who else is moving; one administrator approval covers them all. On the new laptop you choose an account for each, or a new one; a new account's temporary password appears once in the laptop's own window. Their things wait in a private place and land in their own folders when they first sign in, where PCTwin greets them with "Your things are ready". On a Mac this part isn't ready yet, and the app says so.
- **Quick Send works.** Choose files in the system's own picker, pair, and send; the receiver sees what's offered, chooses a folder, and both sides get a receipt.
- **Help without the internet,** a guide for Mac privacy settings, a checklist for the old laptop while you wait, and the laptop stays awake during a move (with a battery warning, and a safe pause when it's nearly empty).
- **Meet your other laptop, then check the connection.** As soon as a move starts, a page shows both laptops (system and PCTwin version), and the next one shows the connection speed measured just now, the free space on the new laptop, and whether each laptop is plugged in. If one laptop has an older PCTwin, the first page says which one to update.
- **A long scan no longer ends the move.** Before, the new laptop gave up after 30 seconds of waiting while the old laptop was still finding files or waiting for a Mac privacy setting or an administrator. Now it waits as long as that takes.
- **Send a folder somewhere else.** On the plan, any folder can go to another place you choose on the new laptop (another drive, say). PCTwin shows how much space is free there, and won't start if it won't fit. If that place stops working, its files are reported as failed. They are never quietly put somewhere else.
- **Undo also removes damaged copies.** If your old drive kept reading a file differently, PCTwin keeps what arrived as a "damaged copy" and tells you. Undo now removes that copy too, unless you changed it since.
- **An independent security review** of the new parts found four things to tighten, and all four are fixed.

## 21 October 2026

**In progress (everyone in one move)**

- **One move can take everyone on a laptop.** With one administrator's approval, PCTwin's helper finds each approved person's files, makes their account on the new laptop, holds their things in a private place during the move, and hands them over into their own folders, as them, the next time they sign in. A person the administrator didn't approve moves nothing.
- **Other people's files stay private.** The person running the move sees other people as counts only ("12 files copied, 2 in use by Bea"), never their file names, and while those files pass through, PCTwin closes itself so the person's other programs can't look inside it.
- **A file copied twice crosses once.** If the same photo sits in two of your folders, the new laptop makes the second copy from the first instead of sending it again. This only happens within one person's things.
- **Try again for files in use.** A file another program had open is never copied half-written; it's listed, and Try again sends just those files.
- **New files are offered at the end.** Files made on the old laptop after you approved the plan are counted when the move ends and offered as a short extra move.
- **PCTwin knows more kinds of places:** mail archives, password vaults, money records, photo, video, music and design projects, code projects, notes, books, game saves, disk images and backups. Things kept only on this laptop move before things a cloud also keeps.
- **A full test of moving everyone** runs on Windows, Mac and Linux test machines with real accounts, about forty files planted in odd places, and a person moving who tries to cheat. It will first run when this stage is finished.
- **Damaged files are handled honestly.** If a file still doesn't match after it was sent again, PCTwin says why: if your old laptop's drive read it differently each time, what arrived is kept beside it as a "damaged copy" (never under the real name) and the report says the old drive may be failing; if the new laptop couldn't store it, it says that. If a drive keeps failing, the move pauses and names it, so you can choose another place.
- **Weak drives are read once.** Pieces cut off by a dropped connection are kept and sent again, not read from the drive again.
- **Stronger checks of "already there".** The new laptop judges for itself, with a key only it knows, so nothing about your files can be learned from what the laptops say to each other. A laptop sending you files you don't own never learns what you already have.
- **Older laptops are recognised.** If the other laptop has an older PCTwin that can't pair, the list says to update it, in all seven languages.
- **Names in every script fit where they can.** On Windows, long names in Chinese, Hindi or Arabic are no longer shortened when they fit.
- **Work VPNs are recognised** on Windows even when they show up as "Ethernet 3".

**In progress**

- **Speed and time left, measured, never guessed.** While a move runs, PCTwin works out its speed from what it really moved in about the last 20 seconds, and the time left from that. In the first few seconds, or when it can't know, it says so instead of guessing. When nothing is moving, it says it is waiting rather than showing a time that keeps growing. This is ready for review; the screens will show it once they are connected to the move.
- **Pause and stop, in the move itself.** Pausing on either laptop now holds the move between two pieces of a file: what is already on its way finishes (never more than 32 MB), the other laptop is told, and nothing is treated as a lost connection. Carrying on picks up exactly where it held. Stopping ends the move at once, even while paused, and a file cut off halfway is never shown as copied. If the connection really is lost during a pause, PCTwin says so. Both laptops need this version of PCTwin for a pause to reach the other laptop. This is ready for review.
- **Undo on a Mac puts copies in the Trash.** Until now, undo on a Mac kept every copy, because a Mac can't prove that no other app is still writing to a file. Now undo moves each copy PCTwin made, and that you haven't changed, into the Trash on the same drive, so nothing is ever deleted and you can put anything back. Before it moves a file, PCTwin first sets the copy aside under a private name and checks that it is the very file it made, still unchanged; if another app swapped in a file of its own, that file goes straight back where it was and never reaches the Trash. A file you edited after the move is always kept. On USB sticks and memory cards (FAT and exFAT drives), undo keeps every copy, because those drives reuse the numbers that tell files apart, and the undo screen will say so before you start. Its tests pass on Mac test machines; it is waiting for an independent review.

## 20 October 2026

**Done**

- **Clearer pairing when someone guesses codes.** After five wrong codes, the old laptop now says plainly that someone on this network tried wrong codes, suggests using your phone's hotspot instead, and waits for you to choose Start again.
- **A fresh code after sleep.** If the old laptop sleeps while showing a code, that code stops working, and a new one shows the moment it wakes.
- **No surprise questions.** Before pairing starts, PCTwin says what a Mac (finding devices on your network) or Windows Firewall is about to ask, and which answer lets your laptops find each other. If a Mac said no, PCTwin says where to switch it back on.

**In progress**

- **The foundation of the administrator helper.** When one move takes everyone on a laptop, a small helper does the few jobs that need an administrator's approval. Its first piece is ready for review: it asks through the computer's own approval window (on Windows and Linux), or as a background service an administrator switches on in System Settings (on a Mac). It talks to PCTwin over a private channel only the person moving can reach, and on every connection it checks which account is calling. If someone says no to the approval window, PCTwin is told plainly and nothing runs.

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
