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
role-body = Open PCTwin on both laptops. Then say which one this is.
role-old-title = My old laptop
role-old-body = Send my files, apps and settings.
role-new-title = My new laptop
role-new-body = Receive my setup from another laptop.
role-note = Nothing moves until you review and approve the plan. Your old laptop stays unchanged during the move.

## Frame parts (footer note, Back, steps)

footer-note-move = Free personal moves
footer-note-send = Free nearby sending
frame-back = Back
# Read out by screen readers only; the step shapes carry the meaning on screen.
frame-step-done = Done
frame-step-current = Now
frame-step-todo = Next
frame-step-position = Step { $number } of { $total }

## F01 Unfinished move

resume-open = Resume your move

## C01 How will you connect?

connect-eyebrow = Your move
connect-title = How will you connect?
connect-body = Whichever way you choose, the steps are the same.
connect-wifi-title = Wi-Fi
connect-wifi-body = Both laptops on the same Wi-Fi.
connect-wifi-status = Available
connect-later = A network cable, the PCTwin bridge cable and moving through an external drive are planned for a later version.
connect-usb-note = An ordinary USB cable cannot connect two laptops for a move.
connect-use-wifi = Use Wi-Fi
# Added: sizes and speeds as measured, shown the same way on every connection screen.
connect-unit-mbps = { NUMBER($value, maximumFractionDigits: 0) } MB/s
connect-unit-gb = { NUMBER($value, maximumFractionDigits: 1) } GB

## C02 Put both laptops together

connect-prep-title = Put both laptops together.
connect-prep-body = Connect them to the same Wi-Fi, then open PCTwin on each.
connect-prep-step-1-title = Join the same Wi-Fi
connect-prep-step-1-body = Copying files does not need the internet.
# $network is the Wi-Fi name this laptop is on.
connect-prep-step-1-network = Network: { $network }.
connect-prep-step-2-title = Allow finding nearby laptops
connect-prep-step-2-body = If Windows or macOS asks for permission, choose Allow.
connect-prep-step-3-title = Choose a role on each laptop
connect-prep-step-3-body = One sends. The other receives.
connect-prep-note = Guest or public Wi-Fi can stop laptops seeing each other.
connect-prep-continue = Continue to pairing

## C06 Meet your other laptop

peer-eyebrow = Your move
peer-title = Meet your other laptop.
peer-body = Both laptops are connected and ready.
peer-sending = Sending
peer-receiving = Receiving
peer-old = Old laptop
peer-new = New laptop
peer-version-label = PCTwin
# $version is the PCTwin version on that laptop, for example "1.0.3".
peer-version = { $version }
peer-status-update = Needs update
peer-status-unsupported = Waiting
peer-ready-note = PCTwin lists your files only after both laptops are paired. If the versions differ, update PCTwin on both.
peer-version-differs = These laptops have different versions of PCTwin. Update PCTwin on both to continue.
peer-version-old-body = Different version: { $version }.
peer-version-new-body = Needs the same version on both laptops.
peer-update-title = Update PCTwin
peer-update-body = Put the same version on both laptops.
# $from and $to are system names, for example "Windows 11" and "macOS". The mockup sentence does not use them.
peer-direction-unsupported = This version of PCTwin cannot move between these two systems yet.
peer-continue = Check the connection

## C07 Let’s check the connection

check-eyebrow = Your move
check-title = Let’s check the connection.
check-body = PCTwin checks speed, space and power to plan the move and estimate the time.
check-measuring = Measuring the connection…
# The value next to it is already formatted, for example "62 MB/s".
check-speed-label = connection speed
check-space-label = free on new laptop
check-link-weak = Weak, so moving may be slow. Moving the laptops closer to the Wi-Fi router can help.
check-power-title = Power
check-power-both = Both laptops connected.
check-power-battery = { $label } is running on battery. Plug it in so a long move is not interrupted.
check-space-title = Storage
check-space-body = { $free } free on the new laptop. Before anything moves, you will see whether your choices fit.
check-internet-title = Internet
check-internet-on = Connected. Copying files does not need the internet.
check-internet-off = Not connected. That is fine: copying files does not need the internet.
check-status-ready = Ready
check-status-plug-in = Plug in
check-status-connected = Available
check-status-not-needed = Not needed
# Added: what is said when something could not be read, so it is never shown as fine.
check-status-unknown = Not known
check-speed-unknown = PCTwin could not measure the connection this time. Check again in a moment.
check-space-unknown = PCTwin could not read the free space on the new laptop.
check-power-unknown = PCTwin could not tell whether { $label } is plugged in.
check-internet-unknown = PCTwin could not tell whether this laptop is online. That is fine: copying files does not need the internet.
check-again = Check again
check-estimate-note = The time estimate is not a fixed promise.
check-continue = Choose what to move

## C09 Waiting for your other laptop

reconnect-title = Waiting for your other laptop.
reconnect-body = Everything copied and checked so far is kept.
# Added: the title of the saved-progress card (the mockup shows Move paused by connection).
reconnect-card-title = Move paused by connection
reconnect-progress =
    .aria-label = Move paused by connection
# $done is an already formatted size, for example "81.6 GB". $total is passed but not shown.
reconnect-saved = { $done } copied and checked so far. The rest is still to come.
# $ago is already formatted, for example "45 seconds ago".
reconnect-last-seen = Last seen { $ago }.
reconnect-status-offline = Offline
reconnect-status-trying = Reconnecting
reconnect-status-pending = Waiting
reconnect-trying = Trying to reconnect by itself… Keep both laptops on, awake and on the same Wi-Fi.
reconnect-expired = Too long has passed to reconnect by itself. Pair again, and PCTwin checks what was already copied.
reconnect-unavailable = { $label } cannot be reached. Check that it is on, awake, has PCTwin open and is on the same Wi-Fi.
reconnect-try-now = Retry connection
reconnect-pair-again = Pair again
reconnect-details = Reconnecting safely
reconnect-details-body = Only these two laptops can reconnect, and each time they use a new private connection.

## C10 The connection has ended (dialog)

ended-title = The connection has ended.
ended-body = Your move is saved. The old code no longer works.
ended-kept-title = Kept
ended-kept-body = The record of your move and everything copied and checked.
ended-not-kept-title = Not kept
ended-not-kept-body = Permission to carry on without pairing again.
ended-pair-again = Pair again
ended-pair-again-body = Connect again, and PCTwin checks what was already copied.
ended-see-moved = Open the saved move
ended-see-moved-body = See what has been copied and checked so far.
ended-home = Return home

## Pairing, shared

pair-back = Back
pair-try-again = Try again
pair-heads-up-title = Your laptop may ask you a question.
pair-heads-up-local-network = Your Mac will ask whether PCTwin can find devices on your local network. Choose Allow, so your two laptops can find each other.
pair-heads-up-firewall = Windows may ask whether PCTwin can accept connections. Choose Allow, so your new laptop can reach this one.
pair-heads-up-continue = Continue

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
pair-old-locked-title = Someone on this network tried wrong codes.
pair-old-locked-body = After five wrong codes, pairing stopped to keep this laptop safe. Nothing was shared. If it was you, choose Start again for a new code.
pair-old-locked-hotspot = If you don’t know who it was, connect both laptops to your phone’s hotspot instead, then choose Start again.
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
pairing-local-network-refused = Your Mac is not letting PCTwin find laptops on this network. If your Mac just asked, choose Allow, then try again. If not, open System Settings, choose Privacy & Security, then Local Network, switch on PCTwin, and try again.
pairing-failed = Pairing did not work this time. Nothing was shared. Try again.

## M06 Match people to their profiles (Who’s moving?)

people-eyebrow = Your move
people-title = Match people to their profiles.
people-body = Choose where each person’s files go on the new laptop.
people-col-old = Old laptop
people-col-new = New laptop
people-col-status = Status
people-you = You
# $name is the person's name on the old laptop. $account is an account name on the new laptop.
people-select =
    .aria-label = Account on the new laptop for { $name }
people-option-existing = { $account } / new laptop
people-option-new = Create a new account
people-option-leave-out = Leave out
people-option-choose = Choose…
people-new-account-hint = The new laptop asks for permission before it creates an account.
people-suggested = Suggested because the names match. Check it is the same person.
# $account is an account name on the new laptop.
people-suggested-use = Use { $account }
people-status-ready = Confirmed
people-status-waiting = Needs their approval
people-status-left-out = Left out
people-status-choose = Needs your choice
people-status-changed = Changed by you
people-waiting-body = { $name } must sign in on the old laptop and approve before their files can move. Until then, their files stay untouched.
people-just-me = Only your own profile was found on the old laptop.
people-changed-note = You changed where someone’s files go. Check it before you continue.
people-shared-title = Shared items
people-shared-body = Shared folders and public documents that belong to everyone on the old laptop.
people-shared-select =
    .aria-label = Where shared items go
people-shared-option-shared = Shared folders on the new laptop
people-shared-option-person = With the files of { $name }
people-shared-option-leave-out = Leave out
people-note = PCTwin never changes anyone’s password or gets round another person’s sign-in.
people-list-label = People on the old laptop
people-todo =
    { $count ->
        [0] Everyone who can move has a place to go.
        [one] { $count } person still needs a choice.
       *[other] { $count } people still need a choice.
    }
people-waiting-count =
    { $count ->
        [one] { $count } person is waiting for approval. You can continue; their files stay untouched until they approve.
       *[other] { $count } people are waiting for approval. You can continue; their files stay untouched until they approve.
    }
people-continue = Continue

## M26 Before you leave this laptop (things to do on the old laptop)

before-eyebrow = Your move
before-title = Before you leave this laptop.
before-body = Check these while you still have your old laptop.
before-licences-title = App licences
before-licences-body = Some apps need you to move their licence yourself.
before-passwords-title = Account recovery
before-passwords-body = Make sure you can still get into your accounts and any locked drives.
before-codes-title = Keep your sign-in codes to hand
before-codes-body = If an account sends you a code or uses an app to confirm it is you, keep that phone or app nearby.
before-online-title = Unsaved work
before-online-body = Save any drafts, and check that files stored online are up to date.
before-done =
    .aria-label = I have done this: { $item }
before-note = Do not type passwords or recovery keys here. Finishing this list does not mean the old laptop is safe to wipe.
before-empty = Nothing needs doing on your old laptop before the move.
before-continue = Return to the move plan

## S11 Get help with this task (a short problem code)

help-eyebrow = Help and privacy
help-title = Get help with this task.
help-body = Share a short code that shows someone what went wrong. Check what it includes first.
help-making = Making a code…
help-code-title = Problem code
# $code is the code itself, for example "PCT-7F3K-21". Keep it unchanged.
help-code =
    .aria-label = Problem code { $code }
help-contains-title = What it includes
help-contains-problem = The kind of problem and how far the task got.
help-contains-step = The step of the move where it happened.
help-contains-versions = The PCTwin version, and which system each laptop runs, such as Windows or macOS.
help-never-title = What it never includes
help-chip-included = Included
help-chip-excluded = Never included
help-never-names = Your name or anyone else’s.
help-never-files = What is in your files, or any passwords.
help-never-network = File or folder names, where your files are kept, or Wi-Fi names.
help-note = The code does not give anyone access to your laptops.
help-copy = Copy code
help-copied = Code copied.
help-share = Share
help-share-hint = Send it by WhatsApp or any app.
help-share-text = My PCTwin problem code is { $code }.
help-failed = A problem code could not be made. Try again. If it still does not work, describe the problem in your own words instead.
help-try-again = Try again
help-copy-failed = The code could not be copied. Select it above and copy it yourself.

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

## M02 Finding your files

scan-eyebrow = Your move
scan-title = Finding your familiar setup.
scan-body = What PCTwin finds is saved, so the search can carry on if it stops.
scan-progress =
    .aria-label = Searching the places you chose
# $size is passed but not shown in the mockup line.
scan-found =
    { NUMBER($count) ->
        [one] { $count } file found so far.
       *[other] { $count } files found so far.
    }
scan-step-usual-title = Files and folders
scan-step-usual-body = Searched the places PCTwin can read.
scan-step-other-title = Other folders and drives
scan-step-other-body = Folders and drives you use outside the usual places.
scan-step-protected-title = Protected places
scan-step-protected-body =
    { NUMBER($count) ->
        [one] { $count } place still needs your permission.
       *[other] { $count } places still need your permission.
    }
scan-step-done = Done
scan-step-current = Now
scan-step-todo = Next
scan-unreadable-note = Places PCTwin could not read stay on the list. They are not counted as searched.
scan-status-not-searched = Not searched
scan-paused = Search paused. Everything found so far is saved.
scan-resumed = The search picked up where it stopped.
scan-partial =
    { NUMBER($count) ->
        [one] { $count } protected place is not searched yet.
       *[other] { $count } protected places are not searched yet.
    }
scan-allow = Allow access
scan-pause = Pause
scan-resume = Continue searching
scan-continue = See what was found

## M03 Everything you chose, in view

overview-eyebrow = Your move
overview-title = Everything you chose, in view.
overview-body = Check what PCTwin found before it makes the plan.
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
overview-files-body = Documents, photos and other folders you chose.
overview-dest-title = Where they go
overview-dest-body = The place on the new laptop for each folder.
overview-people-title = People
overview-people-body = Choose the account on the new laptop for each person.
overview-review = Review
overview-partial =
    { NUMBER($count) ->
        [one] { $count } protected place is not searched yet.
       *[other] { $count } protected places are not searched yet.
    }
overview-allow = Allow access
overview-continue = Continue

## M04 Keep what matters to you

files-eyebrow = Your move
files-title = Keep what matters to you.
files-body = Choose folders, other drives and any other places.
files-search-label = Find a folder
files-search-placeholder = Search folders
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
files-status-added = Added by you
files-add = Add another folder
files-remove =
    .aria-label = Remove { $name }
files-no-limit = Free moves have no limit on file size or how many folders you move.
files-total =
    { NUMBER($count) ->
        [one] { $count } folder chosen, { $size } in total
       *[other] { $count } folders chosen, { $size } in total
    }
files-none = Choose at least one folder to continue.
files-continue = Choose destinations

## M05 Give every file a home

dest-eyebrow = Your move
dest-title = Give every file a home.
dest-body = Choose where each folder goes on the new laptop.
dest-col-chosen = What you chose
dest-col-place = Where it goes
dest-col-placement = Drive
dest-placement-local = Built-in
dest-placement-external = External
dest-col-status = Status
dest-select =
    .aria-label = Destination for { $name }
# $folder is a folder name on the new laptop, for example "Documents".
dest-option-folder = New laptop / { $folder }
# $drive is the name of a drive connected to the new laptop.
dest-option-drive = External drive / { $drive }
dest-option-choose = Choose folder
dest-default-label = Usual place
dest-status-ready = Ready
dest-status-missing = Drive missing
dest-status-space = Not enough space
dest-all-ready = Every folder has a place on the new laptop.
dest-missing = { $drive } is not connected to the new laptop. Connect it, or choose another place for these files.
dest-space = { $place } has { $free } free, but these files need { $needed }. Free up space there, choose another place, or move fewer files.
dest-change = Change
dest-continue = Review the plan

## M23 Approve the move you want

plan-eyebrow = Your move
plan-title = Approve the move you want.
plan-body = PCTwin does only what this plan says.
plan-what-title = What you chose
# $size is already formatted, for example "120 GB". $count is passed but not shown.
plan-what-body = { $size } of files, from the folders you chose, to the places you picked.
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
plan-edit-note = If you change the plan, you approve it again before anything new starts.
plan-approve = I approve this plan, including the things I must do myself.
plan-not-approved = You have not approved this plan yet, so nothing will move.
plan-changed = You changed the plan, so approve it again before anything new moves.
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
scan-card-title = Searching the places you chose
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
progress-count-needs-action =
    { NUMBER($count) ->
        [one] { $count } item needs your action
       *[other] { $count } items need your action
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
progress-compact = Keep it compact
progress-expand = See the whole move
progress-stop = Stop the move

## X02 Move progress, expanded

progress-all-title = See the whole move.
progress-all-body = Each item shows where it comes from and how it is doing.
progress-metric-overall = overall progress
progress-metric-committed = copied and checked
progress-metric-estimate = estimate
progress-col-item = Item
progress-col-from = From
progress-col-status = Status
progress-state-waiting = Waiting
progress-state-copying = Copying
progress-state-verified = Checked
progress-state-checking = Checking
progress-state-paused = Paused
progress-checked-note = An item counts as copied only after it is checked.

## F09 Mini panel

mini-title = Your move, nearby.
mini-title-paused = Move paused.
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
pause-body = Everything copied and checked is kept. The rest waits.
pause-finishing = Finishing { $name } first, so it is not left half-copied.
pause-kept-title = Files
pause-kept-body = Everything copied and checked so far is kept.
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

## X10 Stop the remaining move? (danger dialog)

cancel-title = Stop the remaining move?
cancel-body = Stopping keeps everything copied and checked. It does not undo the move.
cancel-copied-title = Copied files
cancel-copied-body = Stay on the new laptop.
cancel-rest-title = The rest of the move
cancel-rest-body = Nothing new starts. Files not yet copied stay only on the old laptop.
cancel-partial-title = A file partly copied
cancel-partial-body = Is never kept as if it were complete.
cancel-old-title = Old laptop
cancel-old-body = Stays unchanged.
cancel-ack = I understand stopping does not undo what was copied
cancel-stop = Stop and view the report
cancel-keep = Keep moving

## R01 Welcome to your next laptop (outcome)

outcome-eyebrow = Move results
outcome-title = Welcome to your next laptop.
outcome-body = Here is what happened to everything you approved.
outcome-files-title = Selected files
outcome-status-partial = Partly done
outcome-next-title = What to do next
outcome-old-safe = Anything not finished stays listed here. Nothing was removed from your old laptop.
outcome-report = View complete report
outcome-home = Return home

## R02 Your move, item by item (detailed report)

report-title = Your move, item by item.
report-body = What happened to each item, why, and where it came from and went.
report-filter-label = Show
report-col-item = Item
report-col-result = Outcome
report-col-details = Check or next step
report-from = From: { $path }
report-to = To: { $path }
report-none = No items have this result.
report-more-details = More details

# Evidence: how a copied item was checked.
evidence-read-back = Checked by reading it back from the drive.
evidence-memory = Checked from this laptop’s memory, not the drive.
evidence-memory-explain = The copy was read back and matched, but through this laptop’s memory, so what was read may have come from memory rather than the drive. A fault in the drive itself might not show here, so opening the file once is a good check.
evidence-not-checked = Not checked, because it did not finish.
# Why a copied file was checked from memory, one line under the explanation above (the gate's WhyMemory).
why-memory-cannot-skip-cache = This drive would not let PCTwin read it directly, so the check went through this laptop’s memory. Some network drives and USB sticks work this way.
why-memory-not-shown = This drive could not show that PCTwin’s read really reached it, so the check counts as one from this laptop’s memory.
why-memory-no-way = This laptop’s system has no way to read this drive directly, so the check went through its memory.
why-memory-stored-specially = This file is stored in a special way, such as compressed or encrypted, so the system reads it through memory.
why-memory-too-small = This file is very small, so the drive keeps it with its own records and the system reads it through memory.

# Reasons: why an item did not finish. Each says what happened, then what to do.
reason-not-enough-space = The new laptop ran out of space. Free up space on it, then start a new move that includes this item.
reason-changed-during-move = This file kept changing on the old laptop while it was being copied. Close any app using it, then start a new move that includes it.
reason-unreadable = The old laptop would not let PCTwin read this item. Check that it opens on the old laptop, then start a new move that includes it.
reason-not-verified = The copy did not match the original, so it was not kept. Start a new move that includes it. If it fails again, the original may be damaged.
reason-refused-by-safety-check = The new laptop’s safety check refused this item because of its name, place or size. The original is still on the old laptop, so you can copy it another way if you need it.
reason-name-not-allowed = This name cannot be used on the new laptop, and no safe new name was possible. Rename it on the old laptop, then start a new move that includes it.
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
# $minutes is a whole number of minutes, measured, when both ends of the range round to the same number. The result fills "About { $range } left".
progress-minutes-one =
    { NUMBER($minutes) ->
        [one] { $minutes } minute
       *[other] { $minutes } minutes
    }
# $low and $high are numbers and $unit is already formatted, for example "MB/s". The result fills "{ $speed }, measured".
progress-speed-range = { $low } to { $high } { $unit }
progress-count-failed =
    { NUMBER($count) ->
        [one] { $count } item could not be copied
       *[other] { $count } items could not be copied
    }
progress-queue-empty = Nothing has started yet.
progress-approved-job = Approved move
progress-route-old = Old laptop
progress-route-new = New laptop
progress-panel-pausing = Pausing
outcome-metric-size = copied and checked
outcome-metric-action = need your action
progress-percent-unknown = PCTwin is still working out the size of this move.
mini-title-send-paused = Send paused.
cancel-ack-hint = Tick the box above to turn on the stop button.
progress-state-placing = Putting in place
progress-state-existing = Already there
outcome-stopped-title = Your move was stopped.
outcome-stopped-body = You stopped the move. What was copied and checked before then is kept. The rest was not copied.
# $done and $total are counts of items: what was copied and checked, and everything the person approved.
outcome-files-summary = { $done } of { $total } items copied and checked.
outcome-files-memory =
    { NUMBER($count) ->
        [one] { $count } checked from this laptop’s memory, not the drive.
       *[other] { $count } checked from this laptop’s memory, not the drive.
    }
report-show-more = Show more
# $shown and $total are counts of items.
report-showing = Showing { $shown } of { $total }

## Component kit

dialog-close = Close
toast-dismiss = Dismiss
code-label = Six-digit code

## Shared Quick Send words

qs-eyebrow = Quick Send
qs-eyebrow-receiving = Receiving on this laptop
# The route between the two laptops, for example "Blue Fox" to "Green Owl".
qs-route =
    .aria-label = From { $from } to { $to }
qs-route-sender = Sending · this laptop
qs-route-receiver = Receiving · same Wi-Fi
qs-tally =
    { NUMBER($count) ->
        [one] { $count } selected item · { $size }
       *[other] { $count } selected items · { $size }
    }
qs-tally-badge = Nearby send

## Q01 Quick Send home

qs-home-title = A few files. Straight across.
qs-home-body = Send a few files to a laptop nearby.
qs-send-title = Send files
qs-send-body = Choose files or folders on this laptop.
qs-receive-title = Receive files
qs-receive-body = Let a laptop nearby send files to this one.
qs-history-title = Your receipts
qs-history-body = See where received files went and what did not finish.
qs-home-action = Choose files
qs-receiving = Ready to receive. Other laptops nearby see this one as { $label }.
qs-receive-stop = Stop receiving
qs-no-nearby = No laptops found nearby. Open PCTwin on the other laptop and keep both on the same Wi-Fi.

## Q02 Choose files and folders

qs-choose-title = What are you sending?
qs-choose-body = Choose only the files and folders you want to share.
qs-drop =
    .aria-label = Drop area for files and folders
qs-drop-title = Drop files or folders here
qs-drop-body = Or add them with Add files or Add folders.
qs-drop-active = Let go to add them.
qs-add-files = Add files
qs-add-folders = Add folders
qs-remove =
    .aria-label = Remove { $name }
# $kind is the file type, for example "PDF document" or "8 photos".
qs-file-detail = { $size } · { $kind }
qs-empty = Nothing chosen yet. Add files or folders to send.
qs-online-only = Some files are only online. Make them available on this laptop first, then send them.
qs-protected = PCTwin cannot read { $name }. Allow access, or remove it from the list.
qs-allow = Allow access
qs-choose-continue = Check these files

## Q03 Check what you are sharing

qs-check-title = Check what you are sharing.
qs-check-body = Look over these files before they leave this laptop.
qs-check-included = Included
qs-check-shared-note = The other laptop sees these files’ names, sizes and dates.
qs-check-changed = { $name } changed since you chose it. Check it again.
qs-check-ack = I have checked these files
qs-check-ack-hint = Remove anything private before sending.
qs-check-continue = Choose the other laptop

## Q04 Who should receive it?

qs-who-title = Who should receive it?
qs-who-body = Choose the name shown on the other laptop.
qs-who-nearby = Nearby · same Wi-Fi.
qs-who-code = Use a pairing code
qs-who-code-body = Enter the code shown on the sender.
qs-who-same-label = Two laptops nearby have this name. Check the name on the other laptop’s screen.
qs-who-busy = { $label } is busy with another send. Wait a moment, then try again.
qs-who-note = The other laptop accepts the files and chooses where they go.
qs-who-action = Show my send code

## Q08 Incoming offer (receiving laptop)

qs-offer-title = Someone wants to send you files.
qs-offer-body = Check what is being sent before you accept.
qs-offer-note = PCTwin saves only the files you accept. It never opens or runs them.
qs-offer-accept = Accept and choose a folder
qs-offer-decline = Decline these files
qs-offer-declined = You declined. Nothing was saved on this laptop.
qs-offer-changed = The sender changed the files. Check the new list before you accept.
qs-offer-gone = { $label } is no longer available. Nothing was saved on this laptop.

## Q09 Where should the files land?

qs-land-title = Where should the files land?
qs-land-body = Choose the folder on this laptop.
qs-land-folder-label = Receiving folder
qs-land-choose = Choose another folder
qs-land-space-title = Space available
qs-land-space = { $free } available · { $needed } needed.
qs-land-status-fits = Fits
qs-land-status-no-space = Not enough space
qs-land-no-space = This folder has { $free } free, but these files need { $needed }. Choose another folder or free up space.
qs-land-unavailable = This folder is not available. Connect its drive, or choose another folder.
qs-land-denied = PCTwin is not allowed to save in this folder. Choose another folder.
qs-land-note = If a file with the same name is already there, you choose what happens before anything is saved.
qs-land-confirm = Use this folder for these files
qs-land-confirm-hint = Only the files you accepted are saved here.
qs-land-use = Use this folder

## Q10 Keep your existing work safe

qs-conflict-title = Keep your existing work safe.
qs-conflict-body = Choose what happens when a file with the same name is already there.
qs-conflict-col-file = Item
qs-conflict-col-existing = Already there
qs-conflict-col-choice = Choice
# $when is already formatted, for example "yesterday".
qs-conflict-existing = Edited { $when }
qs-conflict-none = No existing copy
qs-conflict-select =
    .aria-label = What happens to { $name }
qs-keep-both = Keep both
qs-skip = Skip this item
qs-add-new = Add new
qs-keep-both-note = Keep both leaves the file already there as it is. The new one is saved as { $newName }.
qs-no-conflicts = None of these files are in the folder yet. Nothing will be replaced.
qs-conflict-target-changed = If the file already there changes or cannot be opened, it waits until you check it again.
qs-conflict-approve = I approve these choices
qs-conflict-continue = Confirm these choices

## Q11 Ready to send these files?

qs-approve-title = Ready to send these files?
qs-approve-body = Check exactly what is being sent before it starts.
qs-approve-receiver-title = Other laptop
qs-approve-receiver-waiting = Waiting for the other laptop to accept and choose a folder.
qs-approve-receiver-ready = The other laptop accepted, and chose a folder and what to do with files already there.
qs-approve-files-title = Files
qs-approve-files-same = Do not change these files until they are sent.
qs-approve-files-changed = Some files changed since you checked them. Check them again.
qs-approve-existing-title = Existing files
qs-approve-existing-body = Keep both, as chosen.
qs-status-waiting = Waiting
qs-status-accepted = Approved
qs-status-unchanged = Review
qs-status-changed = Changed
qs-approve-ack = Send these files
qs-approve-ack-hint = I approve these files and this laptop.
qs-approve-start = Start sending
qs-approve-cleared = You changed the files, so { $label } must accept them again.
qs-approve-withdrawn = { $label } no longer accepts this send. Nothing was sent.

## Q12 Send progress

qs-sending-title = Your files are on their way.
qs-sending-body = Copies go straight from one laptop to the other.
qs-sending-progress =
    .aria-label = Sending selected files
qs-sending-current = Now sending: { $name }
# $speed is measured and already formatted, for example "3.1 MB/s".
qs-sending-speed = { $speed }, measured
qs-step-copying = Copying
qs-step-copying-body =
    { NUMBER($done) ->
        [one] { $done } selected item received.
       *[other] { $done } selected items received.
    }
qs-step-checking = Checking
qs-step-checking-body = Each file is compared with the original.
qs-step-receipt = Receipt
qs-step-receipt-body = At the end, see where files went and what did not finish.
qs-sending-paused = Paused. Files already received and checked are kept.
qs-sending-disconnected = The connection to { $label } dropped. Files already checked are kept. Trying again by itself…
qs-sending-needs-action =
    { NUMBER($count) ->
        [one] { $count } item needs your action.
       *[other] { $count } items need your action.
    }
qs-pause = Pause send
qs-resume = Resume
qs-compact = Keep compact
qs-stop = Stop this send
qs-sending-complete-title = The send is complete
qs-sending-complete-body = Every file that arrived matched its original. See where they were saved.
qs-sending-receipt = See the receipt

## Q14 Handover receipt (both laptops)

qs-receipt-title = Here is your receipt.
qs-receipt-body = See exactly what arrived and where it landed.
qs-receipt-box-title = Your files arrived
qs-receipt-box-body = Each copy was compared with the original and matched.
qs-receipt-arrived =
    { NUMBER($count) ->
        [one] { $count } selected item received
       *[other] { $count } selected items received
    }
qs-receipt-size = { $size } matched the original
qs-receipt-saved-as = { $path }
qs-receipt-not-arrived = Not all of this folder arrived and was checked.
qs-receipt-status-verified = Checked
qs-receipt-status-unfinished = Unfinished
qs-receipt-not-completed = Not completed.
qs-receipt-stopped-row = Received and checked before stopping.
qs-receipt-partial =
    { NUMBER($count) ->
        [one] { $count } item still needs attention.
       *[other] { $count } items still need attention.
    }
qs-receipt-stopped = This send was stopped. Files that fully arrived stay in the folder you chose.
qs-receipt-not-opened = PCTwin did not open or run any received file.
qs-receipt-ack = Acknowledge receipt
# $when is already formatted, for example "today at 14:05".
qs-receipt-acked = Receipt acknowledged { $when }.
qs-receipt-ack-note = Acknowledging this receipt does not confirm the files were opened or used.
qs-receipt-offline = { $label } is offline, so it has not seen this receipt yet. The receipt is saved on this laptop.
qs-receipt-missing = Send missing pieces
qs-receipt-missing-body = Check only the files that did not arrive.
qs-receipt-history = All receipts
qs-receipt-done = Back to Quick Send

## Q19 Finish the missing part

qs-missing-title = Finish the missing part.
qs-missing-body = Use the earlier receipt to see what did not arrive.
qs-missing-status-there = Arrived
qs-missing-status-missing = Needs action
qs-missing-there-body = It arrived and matched the original.
qs-missing-missing-body = Check the files that are missing or did not finish.
qs-missing-note = Files that arrived are kept. New or changed files need your approval again.
qs-missing-ack = Send the missing files
qs-missing-ack-hint = PCTwin checks the files again, and the other laptop accepts them again.
qs-missing-complete = Everything already arrived. There is nothing left to send.
qs-missing-unavailable = The earlier receipt cannot be found, so PCTwin cannot tell what is missing. Start a new send instead.
qs-missing-continue = Check what will be sent

## Q20 Stop this send? (and decline)

qs-stop-title = Stop this send?
qs-stop-body = Check what has already arrived before you stop.
qs-stop-arrived-title = Files that fully arrived
qs-stop-arrived-body = Stay in the folder you chose.
qs-stop-rest-title = Files not sent yet
qs-stop-rest-body = Nothing new is sent after you stop.
qs-stop-partial-title = Files partly arrived
qs-stop-partial-body = Never saved under their real name, so never mistaken for full copies.
qs-stop-status-keep = Keep
qs-stop-status-stop = Stop
qs-stop-status-review = Review
qs-stop-ack = Stop this send
qs-stop-ack-hint = Files that fully arrived are not deleted.
qs-stop-confirm = Stop and view the receipt
qs-stop-keep = Keep sending
qs-decline-title = Decline these files?
qs-decline-body = Nothing is saved on this laptop, and the sender is told you declined.
qs-decline-confirm = Decline
qs-already-finished = This send has already finished. See the receipt for what arrived.

## F08 Your moves, kept on this laptop (history)

history-title = Your moves, kept on this laptop.
history-body = Open a report again, or carry on with a saved move.
history-row-move = { $from } → { $to }
history-row-sent = Sent to { $label }
history-row-received = Received from { $label }
history-status-finished = Completed
history-status-steps-left =
    { NUMBER($count) ->
        [one] Completed, { $count } step for you
       *[other] Completed, { $count } steps for you
    }
history-status-paused = Paused
history-status-stopped = Stopped safely
history-status-partial = Not all arrived
history-open-report = Report
history-open-receipt = Receipt
history-continue = Continue
history-export = Export history
history-exported = History saved as { $path }.
history-export-failed = The history could not be saved there. Choose another folder and try again.
history-note = This history stays on this laptop. Nothing is sent anywhere unless you export it and share it yourself.
history-empty = Your first move and its report will appear here.
history-empty-title = No moves recorded yet
history-start = Start a new move

## Quick Send: words the mockups do not have (added while building; same voice)

qs-soon-note = Quick Send is not switched on in this version yet. Nothing has been sent or received.
qs-tally-counting =
    { NUMBER($count) ->
        [one] { $count } selected item · size still being counted
       *[other] { $count } selected items · size still being counted
    }
qs-gone = { $name } is not there any more. It may have been moved or deleted. Untick it, or put it back and add it again.
qs-choose-why-problems = Some ticked items cannot be sent yet. Untick them, or fix them as described, to continue.
qs-check-missing = { $name } is not there any more. Go back and remove it, or put it back.
qs-check-unreadable = PCTwin cannot read { $name } any more. Allow access, or go back and remove it.
qs-check-show-all =
    { NUMBER($count) ->
        [one] Show the file
       *[other] Show all { $count } files
    }
qs-check-change = Change what I am sending
qs-land-none = No folder chosen yet.
qs-land-space-unknown = PCTwin could not measure the free space here. These files need { $needed }.
qs-conflict-existing-unknown = Already there
qs-conflict-choice-open = Not chosen yet
qs-conflict-why = Choose what happens to every file, then tick the box, to continue.
qs-approve-folder-title = Receiving folder
qs-status-recipient-choice = Recipient choice
qs-sending-card = Sending selected files
qs-progress-of = of { $total }
qs-receipt-partial-title = Not everything arrived
qs-receipt-metric-items =
    { NUMBER($count) ->
        [one] selected item received
       *[other] selected items received
    }
qs-receipt-metric-size = checked against the source
qs-receipt-sent-as = Sent as { $path }
qs-receipt-renamed = A file with that name was already there, so this one has a new name.
qs-receipt-chip-already = Already there
qs-receipt-already = An identical file was already there ({ $path }), so it was not sent again.
qs-receipt-chip-declined = Not accepted
qs-receipt-declined-send = The other laptop did not accept this file, so it was not sent.
qs-receipt-declined-receive = You did not accept this file, so it was not saved.
qs-receipt-more = More details
qs-receipt-empty = Nothing was sent, so there is nothing to show.
qs-receipt-already-count =
    { NUMBER($count) ->
        [one] { $count } item was already there as an identical file
       *[other] { $count } items were already there as identical files
    }
qs-missing-new-send = Start a new send
history-open-name = { $what }: { $title }
