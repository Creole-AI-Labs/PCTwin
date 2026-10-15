# PCTwin screens, English (source language). Other languages are machine-translated ahead of
# time from this file; a string missing in a language falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = PCTwin home
language-label = Language

footer-free = Free personal moves. No account needed.
footer-help = Help

## F01 Welcome

welcome-eyebrow = Your next chapter, ready.
welcome-title-line1 = Your setup.
welcome-title-line2 = Your next laptop.
welcome-body = Bring your files, apps and familiar settings along for the move.
welcome-start = Start a move

## F06 Old or new laptop

role-back = Back
role-eyebrow = Start a move
role-title = Which laptop is this?
role-body = Open PCTwin on both laptops. Choose the role for this one.
role-old-title = My old laptop
role-old-body = Send my files, apps and settings.
role-new-title = My new laptop
role-new-body = Receive my setup from another laptop.
role-note = Nothing moves until you review and approve the plan. Your old laptop stays unchanged during the move.

## Pairing, shared

pair-back = Back
pair-try-again = Try again

## C03 Old laptop: show the code

pair-old-eyebrow = Old laptop
pair-old-title = Let’s connect your laptops.
pair-old-body = On your new laptop, enter the code below.
pair-old-next = Next, you’ll match a number shown on both laptops.
pair-old-network = Keep both laptops on the same Wi-Fi.
pair-old-waiting = Waiting for your new laptop…
pair-old-starting = Getting a code ready…
pair-old-label-title = This laptop’s name for now
pair-old-label-hint = Your new laptop will find this one by this name.
pair-old-shuffle = Choose another name
pair-old-rename = Type a name
pair-old-rename-label = Name for this laptop
pair-old-rename-note = Anyone on this Wi-Fi can see this name.
pair-old-rename-save = Use this name
pair-old-rename-cancel = Cancel
pair-old-code-meta =
    { $seconds ->
        [one] Changes every minute. New code in { $seconds } second.
       *[other] Changes every minute. New code in { $seconds } seconds.
    }
pair-old-steps-title = On your new laptop
pair-old-step-1 = Open PCTwin and choose My new laptop.
pair-old-step-2 = If a list appears, choose this laptop’s name.
pair-old-step-3 = Type the six-digit code shown here.
pair-old-wait-title = A new code is on its way.
pair-old-wait-body =
    { $seconds ->
        [one] That try did not work. A new code appears in { $seconds } second.
       *[other] That try did not work. A new code appears in { $seconds } seconds.
    }
pair-old-locked-title = Pairing stopped.
pair-old-locked-body = There were too many wrong tries, so pairing stopped to keep this laptop safe. Nothing was shared. Start again when you are ready.
pair-old-start-again = Start again
pair-old-timed-out = No number was chosen in time, so here is a new code.
pair-old-declined = The numbers did not match, so nothing was shared. Here is a new code.
pair-old-left = The new laptop stopped pairing before a number was chosen, so nothing was shared. Here is a new code.
pair-network-title = Choose your Wi-Fi.
pair-network-hint = Choose the network your new laptop is on.

## C05 Old laptop: match the number

pair-choose-eyebrow = Confirm the connection
pair-choose-title = Choose the matching number.
pair-choose-body = Look at your new laptop. Choose the number it shows.
pair-choose-number = Number { $number }
pair-choose-confirm = Confirm these laptops
pair-choose-renewed = New numbers are shown. Choose the one on your new laptop.
pair-choose-none = None of these match
pair-choose-note = If the new laptop shows no number, or none of these match it, choose None of these match.
pair-choose-safe = Nothing has been shared yet.
pair-choose-checking = Checking…

## C04 New laptop: find the old laptop and enter the code

pair-new-eyebrow = New laptop
pair-new-searching-title = Looking for your old laptop…
pair-new-searching-body = Keep PCTwin open on your old laptop, on the same Wi-Fi as this one.
pair-new-list-title = Which one is your old laptop?
pair-new-list-body = Choose the name your old laptop shows on its screen.
pair-new-same-label = Another laptop nearby has this name. Check the old laptop’s screen.
pair-new-none-title = No old laptop found.
pair-new-none-body = Check that PCTwin is open on your old laptop and showing a code, and that both laptops are on the same Wi-Fi. Then search again.
pair-new-search-again = Search again
pair-new-code-title = Enter the code.
pair-new-code-body = Type the code shown on { $label }.
pair-new-code-label = Six-digit code
pair-new-code-hint = Read it from your old laptop’s screen. It changes every minute.
pair-new-connect = Connect
pair-new-connecting = Connecting…
pair-new-other-laptop = Choose a different laptop
pair-new-number-eyebrow = Confirm the connection
pair-new-number-title = Choose this number on your old laptop.
pair-new-number-tile = Your number
pair-new-number-label = Your number is { $number }
pair-new-number-body = Your old laptop shows three numbers. Choose this one there.
pair-new-number-waiting = Waiting for your old laptop…

## Paired (both laptops)

pair-done-eyebrow = All set
pair-done-title = Your laptops are paired.
pair-done-body = Your two laptops are connected. Choosing what to move comes next, and it is still being built, so nothing has moved yet.

## Pairing problems (each says what happened, then what to do)

pairing-code-invalid = That code needs six digits. Check the old laptop’s screen and type it again.
pairing-code-wrong = That code did not match. Check the old laptop’s screen and try again.
pairing-code-expired = That code has changed. Type the new one from the old laptop’s screen.
pairing-busy = The old laptop is already pairing with another laptop. Wait a moment, then try again.
pairing-wait = The old laptop is getting a new code ready. Wait for it, then type the new code.
pairing-locked = Pairing stopped after too many wrong tries. On the old laptop, choose Start again.
pairing-unreachable = This laptop could not reach the old laptop. Check that both are on the same Wi-Fi, then try again.
pairing-wrong-number = That number did not match the new laptop’s. Nothing was shared. Type the new code on the new laptop to try again.
pairing-no-match = The numbers did not match on the old laptop. Nothing was shared. Type the new code from the old laptop’s screen.
pairing-timed-out = No number was chosen in time. Try again with the new code on the old laptop.
pairing-connection-lost = The connection between the laptops dropped. Nothing was shared. Try again.
pairing-connection-lost-new = The connection to the old laptop dropped. Nothing was shared.
pairing-different-version = The two laptops have different versions of PCTwin. Update PCTwin on both, then try again.
pairing-name-invalid = That name cannot be used. Try a shorter name with letters and numbers.
pairing-no-network = This laptop is not on a Wi-Fi or wired network. Connect it, then try again.
pairing-choose-network = This laptop is on more than one network. Choose the one your new laptop is on:
pairing-failed = Pairing did not work this time. Nothing was shared. Try again.

## Label words (the name each laptop shows while pairing)

label-colour-red = Red
label-colour-orange = Orange
label-colour-yellow = Yellow
label-colour-green = Green
label-colour-blue = Blue
label-colour-purple = Purple
label-colour-pink = Pink
label-colour-brown = Brown
label-colour-black = Black
label-colour-white = White
label-colour-grey = Grey
label-colour-gold = Gold

label-animal-ant = Ant
label-animal-badger = Badger
label-animal-bat = Bat
label-animal-bear = Bear
label-animal-bee = Bee
label-animal-beetle = Beetle
label-animal-bison = Bison
label-animal-butterfly = Butterfly
label-animal-camel = Camel
label-animal-cat = Cat
label-animal-cheetah = Cheetah
label-animal-chicken = Chicken
label-animal-crab = Crab
label-animal-crane = Crane
label-animal-crocodile = Crocodile
label-animal-crow = Crow
label-animal-deer = Deer
label-animal-dolphin = Dolphin
label-animal-donkey = Donkey
label-animal-dove = Dove
label-animal-dragonfly = Dragonfly
label-animal-duck = Duck
label-animal-eagle = Eagle
label-animal-elephant = Elephant
label-animal-falcon = Falcon
label-animal-flamingo = Flamingo
label-animal-fox = Fox
label-animal-frog = Frog
label-animal-gazelle = Gazelle
label-animal-swordfish = Swordfish
label-animal-giraffe = Giraffe
label-animal-goat = Goat
label-animal-goose = Goose
label-animal-gorilla = Gorilla
label-animal-hamster = Hamster
label-animal-hawk = Hawk
label-animal-hedgehog = Hedgehog
label-animal-armadillo = Armadillo
label-animal-hippo = Hippo
label-animal-horse = Horse
label-animal-hummingbird = Hummingbird
label-animal-jaguar = Jaguar
label-animal-jellyfish = Jellyfish
label-animal-kangaroo = Kangaroo
label-animal-kingfisher = Kingfisher
label-animal-koala = Koala
label-animal-ladybird = Ladybird
label-animal-lemur = Lemur
label-animal-leopard = Leopard
label-animal-lion = Lion
label-animal-lizard = Lizard
label-animal-llama = Llama
label-animal-lobster = Lobster
label-animal-lynx = Lynx
label-animal-macaw = Macaw
label-animal-meerkat = Meerkat
label-animal-mole = Mole
label-animal-moose = Moose
label-animal-pufferfish = Pufferfish
label-animal-mouse = Mouse
label-animal-octopus = Octopus
label-animal-ostrich = Ostrich
label-animal-otter = Otter
label-animal-owl = Owl
label-animal-panda = Panda
label-animal-kiwi = Kiwi
label-animal-parrot = Parrot
label-animal-peacock = Peacock
label-animal-pelican = Pelican
label-animal-penguin = Penguin
label-animal-puffin = Puffin
label-animal-rabbit = Rabbit
label-animal-raccoon = Raccoon
label-animal-rhino = Rhino
label-animal-robin = Robin
label-animal-salmon = Salmon
label-animal-seahorse = Seahorse
label-animal-seal = Seal
label-animal-shark = Shark
label-animal-sheep = Sheep
label-animal-sloth = Sloth
label-animal-snail = Snail
label-animal-sparrow = Sparrow
label-animal-squirrel = Squirrel
label-animal-starfish = Starfish
label-animal-stork = Stork
label-animal-swan = Swan
label-animal-tiger = Tiger
label-animal-snake = Snake
label-animal-toucan = Toucan
label-animal-turtle = Turtle
label-animal-walrus = Walrus
label-animal-whale = Whale
label-animal-wolf = Wolf
label-animal-woodpecker = Woodpecker
label-animal-yak = Yak
label-animal-zebra = Zebra
label-animal-dog = Dog
label-animal-cow = Cow
label-animal-beaver = Beaver

# Translators: the laptop's name is a colour plus an animal, for example "Blue Fox".
# Word order, gender and agreement are yours to choose for your language.
label-picked = { $colour } { $animal }
label-unnamed = Unnamed laptop

## Tray menu

tray-show = Show PCTwin
tray-quit = Quit PCTwin

## F01 secondary actions and their coming-soon pages

welcome-restore = Restore setup
welcome-rescue = Rescue a laptop
welcome-quick-send = Quick Send

soon-eyebrow = Coming soon
soon-restore-title = Restore setup
soon-restore-body = Rebuild a laptop from a setup profile you saved after a move. This is coming in a later version.
soon-rescue-title = Rescue a laptop
soon-rescue-body = Bring files back from a laptop that will not start, by connecting its drive to a working one. This is coming in a later version.
soon-quick-send-title = Quick Send
soon-quick-send-body = Send chosen files or folders to a laptop nearby, at any time, not only during a move. It is free, with no account, and it is coming in the first release.

## X01 Move progress, compact

progress-eyebrow = Your move
progress-title = Settling you in.
progress-title-paused = Move paused
progress-paired = Paired
progress-percent =
    .aria-label = Move progress
# $done and $total are already formatted sizes, for example "81.6 GB" and "120 GB".
progress-amount = { $done } of { $total }
# $range is a measured range, already formatted, for example "15 to 20 minutes".
progress-time-left = About { $range } left
progress-time-measuring = Working out the time left…
# $speed is measured and already formatted, for example "62 MB/s".
progress-speed = { $speed }, measured
progress-count-copied =
    { NUMBER($count) ->
        [one] { $count } item copied and checked
       *[other] { $count } items copied and checked
    }
progress-count-checking =
    { NUMBER($count) ->
        [one] { $count } item being checked
       *[other] { $count } items being checked
    }
progress-count-needs-action =
    { NUMBER($count) ->
        [one] { $count } item needs your action
       *[other] { $count } items need your action
    }
progress-count-waiting =
    { NUMBER($count) ->
        [one] { $count } item still to copy
       *[other] { $count } items still to copy
    }
progress-current-copying = Now copying: { $name }
progress-current-checking = Now checking: { $name }
progress-needs-action =
    { NUMBER($count) ->
        [one] { $count } item needs your action. The rest keeps moving.
       *[other] { $count } items need your action. The rest keeps moving.
    }
progress-see-needs-action = See what needs you
progress-interrupted = The connection to the other laptop dropped. PCTwin is trying to reconnect. Nothing copied and checked is lost.
progress-low-battery = { $label } is low on battery. Plug it in so the move is not interrupted.
progress-old-safe = Your old laptop stays unchanged.
progress-pause = Pause move
progress-resume = Resume move
progress-compact = Keep it compact
progress-expand = See the whole move
progress-stop = Stop the move

## X02 Move progress, expanded

progress-all-title = See the whole move.
progress-all-body = Each operation keeps its own status and source.
progress-metric-overall = overall progress
progress-metric-committed = committed so far
progress-metric-estimate = estimate
progress-col-item = Operation
progress-col-from = Source
progress-col-status = State
progress-state-waiting = Waiting
progress-state-copying = Copying
progress-state-verified = Verified
progress-state-checking = Checking
progress-state-paused = Paused
progress-checked-note = An item counts as copied only after it is checked.
progress-collapse = Show less

## F09 Mini panel

mini-title = Your move, nearby.
mini-title-paused = Move paused.
mini-body = The full window is one tap away.
mini-what = moving your selected files
mini-progress =
    .aria-label = Move progress
mini-pause = Pause
mini-resume = Resume
mini-open = Open move
# Used when lane E reuses this panel for a Quick Send.
mini-title-send = Your files are on their way.
mini-what-send = sending selected files
mini-open-send = Open send
mini-progress-send =
    .aria-label = Send progress

## X05 Paused at a safe point

pause-title = Paused at a safe point.
pause-title-finishing = Pausing at a safe point…
pause-body = Committed results stay recorded and pending actions wait.
pause-finishing = Finishing { $name } first, so it is not left half-copied.
pause-kept-title = Files
pause-kept-body = Block checkpoint preserved.
pause-waiting-title = Waiting
pause-waiting-body =
    { NUMBER($count) ->
        [one] { $count } item still to copy. Nothing is lost.
       *[other] { $count } items still to copy. Nothing is lost.
    }
pause-status-kept = Paused
pause-status-waiting = Waiting
pause-note = Pausing does not undo anything. Your old laptop stays unchanged.
pause-resume = Resume saved move
pause-stop = Stop the move

## X10 Stop the remaining move? (danger dialog)

cancel-title = Stop the remaining move?
cancel-body = Cancellation keeps committed data. It is separate from undo.
cancel-copied-title = Copied files
cancel-copied-body = Stay on the new laptop.
cancel-rest-title = Future actions
cancel-rest-body = Stop after safe checkpoint handling.
cancel-partial-title = A file partly copied
cancel-partial-body = Is never kept as if it were complete.
cancel-old-title = Old laptop
cancel-old-body = Remains untouched by cancellation.
cancel-ack = I understand cancellation does not reverse the move
cancel-stop = Stop and view the report
cancel-keep = Keep moving

## R01 Welcome to your next laptop (outcome)

outcome-eyebrow = Move results
outcome-title = Welcome to your next laptop.
outcome-body = Here is what happened to the scope you approved.
# $size is already formatted, for example "118 GB".
outcome-size = { $size } verified copies
outcome-files-title = Selected files
outcome-files-body =
    { NUMBER($count) ->
        [one] Captured bytes verified; { $count } location still excluded.
       *[other] Captured bytes verified; { $count } locations still excluded.
    }
outcome-status-partial = Partial
outcome-copied =
    { NUMBER($count) ->
        [one] { $count } item copied
       *[other] { $count } items copied
    }
outcome-reinstalled =
    { NUMBER($count) ->
        [one] { $count } item reinstalled
       *[other] { $count } items reinstalled
    }
outcome-converted =
    { NUMBER($count) ->
        [one] { $count } item converted
       *[other] { $count } items converted
    }
outcome-needs-action =
    { NUMBER($count) ->
        [one] { $count } needs your action
       *[other] { $count } need your action
    }
outcome-skipped-by-user =
    { NUMBER($count) ->
        [one] { $count } item skipped by you
       *[other] { $count } items skipped by you
    }
outcome-failed =
    { NUMBER($count) ->
        [one] { $count } item failed
       *[other] { $count } items failed
    }
outcome-all-done = Every item you chose was copied and checked.
outcome-unfinished =
    { NUMBER($count) ->
        [one] { $count } item did not finish. The report says why and what to do next.
       *[other] { $count } items did not finish. The report says why and what to do next.
    }
outcome-next-title = What to do next
outcome-next-needs-action = Finish the steps marked Needs action. Each one says what to do.
outcome-next-failed = Look at the items marked Failed. Each one says what happened and how to try again.
outcome-next-check = Open a few of your files here to make sure they are as you expect.
outcome-old-safe = Unresolved selected work stays visible. The source has not been cleaned or wiped.
outcome-report = View complete report
outcome-home = Return home

## R02 Your move, item by item (detailed report)

report-title = Your move, item by item.
report-body = A factual local record with reasons and source-to-target mappings.
report-filter-label = Show
report-filter-all = All
report-count =
    { NUMBER($count) ->
        [one] { $count } item
       *[other] { $count } items
    }
report-col-item = Item
report-col-result = Outcome
report-col-details = Evidence / next step
report-from = From: { $path }
report-to = To: { $path }
report-none = No items have this result.
report-more-details = More details
report-back = Back

# Evidence: how a copied item was checked.
evidence-read-back = Checked by reading it back from the drive.
evidence-memory = Checked from this laptop’s memory, not the drive.
evidence-memory-explain = The copy was read back and matched, but through this laptop’s memory, so what was read may have come from memory rather than the drive. A fault in the drive itself might not show here, so opening the file once is a good check.
evidence-not-checked = Not checked, because it did not finish.

# Reasons: why an item did not finish. Each says what happened, then what to do.
reason-not-enough-space = The new laptop ran out of space. Free up space on it, then move this item again.
reason-changed-during-move = This file kept changing on the old laptop while it was being copied. Close any app using it, then move it again.
reason-unreadable = The old laptop would not let PCTwin read this item. Check that it opens on the old laptop, then move it again.
reason-not-verified = The copy did not match the original, so it was not kept. Move it again; if it fails again, the original may be damaged.
reason-refused-by-safety-check = The new laptop’s safety check refused this item because of its name, place or size. The original is still on the old laptop, so you can copy it another way if you need it.
reason-name-not-allowed = This name cannot be used on the new laptop, and no safe new name was possible. Rename it on the old laptop, then move it again.
reason-install-failed = The app did not install. Install it from its maker’s website.
reason-no-equivalent = There is no version of this app for the new laptop’s system. Look for a similar app that works there.
reason-sign-in = It is in place, but you need to sign in. Open it and sign in.
reason-activation = The app is in place, but it needs its licence on the new laptop. Open it and activate it with your licence or account.
reason-permission = The new laptop needs to give permission first. Open the item and choose Allow when asked.
reason-online-only = This file is stored online only, so it was not on the old laptop to copy. Download it on the new laptop from the app that stores it online.
reason-manual-install = This app must come from its maker’s website. Download and install it there.
reason-restart = The new laptop must restart to finish. Restart it when you are ready.
reason-finish-by-hand = PCTwin cannot do this step for you. Follow the steps shown for it in this report.

## Item results (every item ends with one of these)

status-copied = Copied
status-reinstalled = Reinstalled
status-converted = Converted
status-needs-action = Needs action
status-skipped-by-user = Skipped by you
status-failed = Failed

## Screens: words the copy deck did not cover (same voice)

# $low and $high are whole minutes, a measured range. The result fills "About { $range } left".
progress-minutes = { $low } to { $high } minutes
# $low and $high are numbers and $unit is already formatted, for example "MB/s". The result fills "{ $speed }, measured".
progress-speed-range = { $low } to { $high } { $unit }
progress-count-failed =
    { NUMBER($count) ->
        [one] { $count } item could not be copied
       *[other] { $count } items could not be copied
    }
progress-queue-empty = Nothing has started yet.
progress-panel-pausing = Pausing
outcome-metric-size = copied and checked
outcome-metric-done = items in place
outcome-metric-action = need your action
progress-percent-unknown = PCTwin is still working out the size of this move.
mini-title-send-paused = Send paused.
cancel-ack-hint = Tick the box above to turn on the stop button.
outcome-not-copied-chip = Not copied
progress-state-placing = Putting in place
progress-state-existing = Already there
outcome-stopped-title = Your move was stopped.
outcome-stopped-body = You stopped the move. What was copied and checked before then is kept. The rest was not copied.
outcome-not-copied =
    { NUMBER($count) ->
        [one] { $count } item was not copied
       *[other] { $count } items were not copied
    }
outcome-checked-memory =
    { NUMBER($count) ->
        [one] { $count } file was checked from this laptop’s memory, not the drive. It was still compared with the original, and the report marks it.
       *[other] { $count } files were checked from this laptop’s memory, not the drive. They were still compared with the originals, and the report marks each one.
    }
report-show-more = Show more
# $shown and $total are counts of items.
report-showing = Showing { $shown } of { $total }
report-memory-summary =
    { NUMBER($count) ->
        [one] { $count } file was checked from this laptop’s memory, not the drive. It is marked below.
       *[other] { $count } files were checked from this laptop’s memory, not the drive. Each one is marked below.
    }

## Component kit

dialog-close = Close
toast-dismiss = Dismiss
code-label = Six-digit code
