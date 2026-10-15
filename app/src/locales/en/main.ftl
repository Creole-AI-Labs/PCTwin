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

## M02 Finding your familiar setup

scan-eyebrow = Reviewed move
scan-title = Finding your familiar setup.
scan-body = The inventory is saved so the scan can resume.
scan-progress =
    .aria-label = Scanning selected locations
# $size is passed but not shown in the mockup line.
scan-found =
    { NUMBER($count) ->
        [one] Scan: { $count } file discovered.
       *[other] Scan: { $count } files discovered.
    }
scan-step-usual-title = Files and folders
scan-step-usual-body = Accessible selected paths scanned.
scan-step-other-title = Other folders and drives
scan-step-other-body = Folders and drives you use outside the usual places.
scan-step-protected-title = Protected locations
scan-step-protected-body =
    { NUMBER($count) ->
        [one] { $count } location still needs permission.
       *[other] { $count } locations still need permission.
    }
scan-step-done = Done
scan-step-current = Now
scan-step-todo = Next
scan-unreadable-note = Unreadable locations remain visible. They are not counted as fully scanned.
scan-status-not-searched = Not scanned
scan-paused = Scan paused. Everything found so far is saved.
scan-resumed = The scan picked up where it stopped.
scan-partial =
    { NUMBER($count) ->
        [one] Partial scan: { $count } protected location is still excluded.
       *[other] Partial scan: { $count } protected locations are still excluded.
    }
scan-allow = Allow access
scan-pause = Pause
scan-resume = Continue scanning
scan-continue = Preview inventory

## M03 Everything you chose, in view

overview-eyebrow = Reviewed move
overview-title = Everything you chose, in view.
overview-body = Review the inventory before creating the move plan.
# The metric value next to each label is passed in already formatted.
overview-size-label = selected files
overview-items-label =
    { NUMBER($count) ->
        [one] file
       *[other] files
    }
overview-attention-label =
    { NUMBER($count) ->
        [one] needs your attention
       *[other] need your attention
    }
overview-files-title = Files and folders
overview-files-body = Documents, photos and selected custom locations.
overview-dest-title = Where they go
overview-dest-body = The place on the new laptop for each folder.
overview-people-title = Profiles
overview-people-body = Choose the receiving account for each.
overview-review = Review
overview-partial =
    { NUMBER($count) ->
        [one] Partial scan: { $count } protected location is still excluded.
       *[other] Partial scan: { $count } protected locations are still excluded.
    }
overview-allow = Allow access
overview-continue = Continue

## M04 Keep what matters to you

files-eyebrow = Reviewed move
files-title = Keep what matters to you.
files-body = Choose folders, secondary drives and custom locations.
files-search-label = Find a local folder
files-search-placeholder = Search selected paths
files-select =
    .aria-label = Move { $name }
files-folder-detail =
    { NUMBER($count) ->
        [one] { $size } · { $count } file
       *[other] { $size } · { $count } files
    }
files-status-protected = Needs permission
files-protected-body = PCTwin needs your permission to read this folder. Its files are not counted until you allow it.
files-allow = Allow access
files-status-added = Custom path
files-add = Add another folder
files-remove =
    .aria-label = Remove { $name }
files-no-limit = The free core has no arbitrary file-size or folder-count cap.
files-total =
    { NUMBER($count) ->
        [one] { $count } folder chosen, { $size } in total
       *[other] { $count } folders chosen, { $size } in total
    }
files-none = Choose at least one folder to continue.
files-continue = Choose destinations

## M05 Give every file a home

dest-eyebrow = Reviewed move
dest-title = Give every file a home.
dest-body = Map selected categories to a target you control.
dest-col-chosen = Selected data
dest-col-place = Destination
dest-col-placement = Placement
dest-placement-local = Local
dest-placement-external = External
dest-col-status = Status
dest-select =
    .aria-label = Destination for { $name }
# $folder is a folder name on the new laptop, for example "Documents".
dest-option-folder = New laptop / { $folder }
# $drive is the name of a drive connected to the new laptop.
dest-option-drive = External disk / { $drive }
dest-option-choose = Choose folder
dest-default-label = Default destination
dest-status-ready = Ready
dest-status-missing = Drive missing
dest-status-space = Not enough space
dest-all-ready = Every folder has a place on the new laptop.
dest-missing = { $drive } is not connected to the new laptop. Connect it, or choose another place for these files.
dest-space = { $place } has { $free } free, but these files need { $needed }. Free up space there, choose another place, or move fewer files.
dest-change = Change
dest-continue = Review the plan

## M23 Approve the move you want

plan-eyebrow = Reviewed move
plan-title = Approve the move you want.
plan-body = This versioned plan is the only authority for execution.
plan-what-title = Selected scope
# $size is already formatted, for example "120 GB". $count is passed but not shown.
plan-what-body = { $size } of files. Selected folders and mapped destinations.
plan-where-title = Where it goes
plan-where-body =
    { NUMBER($count) ->
        [one] To { $count } place on the new laptop.
       *[other] To { $count } places on the new laptop.
    }
plan-hand-title = Before you start
plan-hand-body =
    { NUMBER($count) ->
        [one] { $count } step. It is listed in your report at the end.
       *[other] { $count } steps. They are listed in your report at the end.
    }
plan-hand-none = Nothing, as far as PCTwin can tell now.
plan-old-safe = Your old laptop stays unchanged. Files are copied, never taken off it.
plan-edit = Change the plan
plan-edit-note = Editing the plan invalidates approval for new operations.
plan-approve = I approve this reviewed plan and its disclosed manual steps.
plan-not-approved = The draft plan has not been approved. New executable operations remain blocked.
plan-changed = These edits invalidate the previous approval for new operations.
plan-start = Start the move

## Plan screens: shared words and the lines the copy deck did not cover

plan-back = Back
# $n is a count of files or folders.
plan-number = { NUMBER($n) }
# $n is a number already rounded for reading, for example 18.4.
plan-size-b = { NUMBER($n) } bytes
plan-size-kb = { NUMBER($n) } KB
plan-size-mb = { NUMBER($n) } MB
plan-size-gb = { NUMBER($n) } GB
plan-size-tb = { NUMBER($n) } TB
# $size is a size already written out, for example "18.4 GB". Used when part of a folder could not be read.
plan-size-at-least = at least { $size }
plan-size-unknown = size unknown
plan-role-home = Home folder
plan-role-desktop = Desktop
plan-role-documents = Documents
plan-role-downloads = Downloads
plan-role-pictures = Pictures
plan-role-music = Music
plan-role-videos = Videos
plan-role-public = Public folder
plan-role-app-data = App data
plan-role-other = Other folder
# $done and $total are counts of places this search has looked at and has to look at.
scan-places = { $done } of { $total } places searched
scan-card-title = Scanning selected locations
scan-pausing = Pausing after the folder it is on. Everything found so far is saved.
overview-row-attention = Needs attention
files-status-work = Work or school
files-work-body = Your work or school manages this folder, so it is left out unless you tick it.
# $count is how many files in the folder are only online.
files-online-only =
    { NUMBER($count) ->
        [one] { $count } file is only online, so it is not copied.
       *[other] { $count } files are only online, so they are not copied.
    }
files-no-match = No folder matches that name.
dest-status-none = Choose a place
dest-status-readonly = Cannot be written to
dest-status-format = File too big
# $name is the folder that needs a place.
dest-none = Choose a place for { $name } on the new laptop.
# $place is a drive on the new laptop that cannot be written to.
dest-readonly = { $place } is read-only, so nothing can be copied to it. Choose another place.
# $place is the drive. Some drive formats cannot hold a single very large file.
dest-format = A file in these folders is too big for the way { $place } is set up. Choose another place.
# $folder is a folder on the new laptop and $drive the drive it is on.
plan-place = { $folder } on { $drive }
plan-group-to = Goes to { $place }
plan-version = Plan version { $revision }
plan-details = More details
# $reference is a short code that names exactly this version of the plan.
plan-reference = Reference { $reference }
# $count is how many files are affected in each line below.
plan-hand-cloud-only =
    { NUMBER($count) ->
        [one] { $count } file is only online. PCTwin does not download it. Make it available on the old laptop first, or leave it.
       *[other] { $count } files are only online. PCTwin does not download them. Make them available on the old laptop first, or leave them.
    }
plan-hand-unreadable =
    { NUMBER($count) ->
        [one] { $count } item could not be opened, so it is not moving. Allow access, or leave it.
       *[other] { $count } items could not be opened, so they are not moving. Allow access, or leave them.
    }
plan-hand-work-managed =
    { NUMBER($count) ->
        [one] { $count } item is managed by your work or school and is left out. Tick its folder to include it.
       *[other] { $count } items are managed by your work or school and are left out. Tick their folders to include them.
    }
plan-blocked-someone-unmapped = Someone on the old laptop has no account chosen on the new one yet. Choose where each person's files go, then come back.
plan-blocked-account-used-twice = Two people would end up in one account on the new laptop. Give each person their own.
plan-blocked-invalid = This plan cannot be approved as it is. Go back and check what you chose.
plan-blocked-fix = Choose accounts

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

## Item results (every item ends with one of these)

status-copied = Copied
status-reinstalled = Reinstalled
status-converted = Converted
status-needs-action = Needs action
status-skipped = Skipped
status-failed = Failed

## Component kit

dialog-close = Close
toast-dismiss = Dismiss
code-label = Six-digit code
