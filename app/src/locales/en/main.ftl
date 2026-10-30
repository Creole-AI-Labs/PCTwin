# PCTwin screens, English (source language). Other languages are machine-translated ahead of
# time from this file; a string missing in a language falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = PCTwin home
language-label = Language
motion-label = Motion
motion-system = Motion: follow system
motion-reduced = Motion: reduced
motion-off = Motion: off

# The opening (Wordplay); its tagline is the home headline.
opening-label = PCTwin is opening
opening-status = Opening PCTwin.
opening-skip = Skip intro
opening-tagline = Send files, or move everything.

footer-free = Free. No account needed.
footer-help = Help

## F01 Home

welcome-title-line1 = Send files,
welcome-title-line2 = or move everything.
welcome-body = Send a few files to a laptop nearby, or bring your files, apps and settings to your new laptop.
welcome-start = Start a full move

## F01 Home: the cards, My history, a move to resume, files waiting

home-quick-send-body = Send or receive files
home-move-body = Move your setup
home-restore-body = Rebuild from a saved setup
home-rescue-body = Copy files from a laptop that won’t start
home-history = My history
home-history-body = Your moves and their reports
name-title = Name this laptop
name-body = Other laptops nearby will find this one by this name. You can change it later, when you pair.
name-use = Use this name
home-update-title = A new version is available
home-update-action = Update now
home-updating = Updating. PCTwin will close and open again by itself.
home-update-failed = The update could not be installed. PCTwin works as before; it will offer the update again next time.
home-update-busy = Finish your move first. Then update PCTwin.
home-resume-body = Your progress is saved.
home-resume-action = Resume
    .aria-label = Resume your move
home-waiting =
    { NUMBER($count) ->
        [one] { $count } file waiting for { $to }
       *[other] { $count } files waiting for { $to }
    }

## Header: your laptop, then the language and motion controls (Help is in the footer)

header-this-laptop = Your laptop

## F06 Old or new laptop

role-back = Back
role-eyebrow = Start a full move
role-title = Which laptop is this?
role-body = Open PCTwin on both laptops. Then say which one this is.
role-old-title = This is my old laptop
role-old-body = Send my files, apps and settings.
role-new-title = This is my new laptop
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

connect-prep-title = Put both laptops together
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
peer-title = Check both laptops
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

## C07 Check the connection

check-eyebrow = Your move
check-title = Check the connection
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
check-speed-unknown = PCTwin could not measure the connection this time. Files still move; only the time estimate is missing.
check-space-unknown = PCTwin could not read the free space on the new laptop.
check-power-unknown = PCTwin could not tell whether { $label } is plugged in.
check-internet-unknown = PCTwin could not tell whether this laptop is online. That is fine: copying files does not need the internet.
check-again = Check again
check-estimate-note = The time estimate is not a fixed promise.
check-continue = Choose what to move
check-continue-new = Continue
# Added 23 October 2026: the short checklist both laptops show before the move.
check-before-title = Before the move starts
check-before-lids-title = Keep both lids open
check-before-lids-body = A closed lid can put a laptop to sleep, which pauses the move.
check-before-near-title = Stay near the Wi-Fi router
check-before-near-body = Keep both laptops close to it while your things move.

## C09 Waiting for your other laptop

reconnect-title = Waiting for your other laptop
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
# Shown with Stop when the tries to reconnect keep bringing nothing (the Task List's reconnect loop).
reconnect-nothing-new = Nothing new has arrived for a while. PCTwin keeps trying by itself, or you can stop the move. Everything copied and checked so far stays.
reconnect-details = Reconnecting safely
reconnect-details-body = Only these two laptops can reconnect, and each time they use a new private connection.

## C10 The connection has ended (dialog)

ended-title = The connection has ended
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
pair-more-details = More details
pair-heads-up-title = Your laptop may ask you a question
pair-heads-up-local-network = Your Mac will ask whether PCTwin can find devices on your local network. Choose Allow, so your two laptops can find each other.
pair-heads-up-firewall = Windows may ask whether PCTwin can accept connections. Choose Allow, so your new laptop can reach this one.
pair-heads-up-continue = Continue

## C03 Old laptop: show the code

pair-old-eyebrow = Old laptop
pair-old-title = Connect your laptops
pair-old-body = On your new laptop, enter the code below.
pair-old-next = Next, you’ll match a number shown on both laptops.
pair-old-network = Keep both laptops on the same Wi-Fi.
pair-old-waiting = Waiting for your new laptop…
pair-old-starting = Getting a code ready…
pair-old-label-title = This laptop’s name
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
pair-old-wait-title = Wait for a new code
pair-old-wait-body =
    { $seconds ->
        [one] That try did not work. A new code appears in { $seconds } second.
       *[other] That try did not work. A new code appears in { $seconds } seconds.
    }
pair-old-locked-title = Someone on this network tried wrong codes
pair-old-locked-body = After five wrong codes, pairing stopped to keep this laptop safe. Nothing was shared. If it was you, choose Start again for a new code.
pair-old-locked-hotspot = If you don’t know who it was, connect both laptops to your phone’s hotspot instead, then choose Start again.
pair-old-start-again = Start again
pair-old-timed-out = No number was chosen in time, so here is a new code.
pair-old-declined = The numbers did not match, so nothing was shared. Here is a new code.
pair-old-left = The new laptop stopped pairing before a number was chosen, so nothing was shared. Here is a new code.
pair-network-title = Choose your Wi-Fi
pair-network-hint = Choose the network your new laptop is on.

## C05 Old laptop: match the number

pair-choose-eyebrow = Confirm the connection
pair-choose-title = Choose the matching number
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
pair-new-search-failed-title = Could not look for your old laptop
pair-new-list-title = Which one is your old laptop?
pair-new-list-body = Choose the name your old laptop shows on its screen.
pair-new-same-label = Another laptop nearby has this name. Check the old laptop’s screen.
pair-new-needs-update = This laptop has an older PCTwin. Update PCTwin on it, then search again.
pair-new-none-title = No old laptop found
pair-new-none-body = Check that PCTwin is open on your old laptop and showing a code, and that both laptops are on the same Wi-Fi. Then search again.
pair-new-search-again = Search again
pair-old-same-role = Another laptop nearby is also set up as the old laptop. One of the two must be the new laptop.
pair-old-switch = This is the new laptop
pair-new-none-same-role = Did the other laptop also choose New laptop? Then choose Old laptop on it, or switch this one.
pair-new-switch = This is the old laptop
pair-new-code-title = Enter the code
pair-new-code-body = Type the code shown on { $label }.
pair-new-code-label = Six-digit code
pair-new-code-hint = Read it from your old laptop’s screen. It changes every minute.
pair-new-connect = Connect
pair-new-connecting = Connecting…
pair-new-other-laptop = Choose a different laptop
pair-new-number-eyebrow = Confirm the connection
pair-new-number-title = Choose this number on your old laptop
pair-new-number-tile = Your number
pair-new-number-label = Your number is { $number }
pair-new-number-body = Your old laptop shows three numbers. Choose this one there.
pair-new-number-waiting = Waiting for your old laptop…

## Paired (both laptops)

pair-done-eyebrow = Paired
pair-done-title = Your laptops are paired
pair-done-body = Your two laptops are connected. Next, your old laptop finds your files, and you approve what moves on the new laptop. Nothing moves before you approve.
pair-done-continue = Continue

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
pairing-local-network-refused = Your Mac may be stopping PCTwin from finding laptops on this network. If your Mac just asked, choose Allow, then try again. If not, open System Settings, choose Privacy & Security, then Local Network, switch on PCTwin, and try again.
pairing-local-network-refused-details = A VPN, or a firewall or filter app, can also stop laptops finding each other. If one is on, turn it off while you pair, then try again.
pairing-failed = Pairing did not work this time. Nothing was shared. Try again.
pairing-failed-new = Pairing did not work this time. Nothing was shared. Type the new code from the old laptop’s screen.

## M06 Match people to their profiles (Who’s moving?)

people-eyebrow = Your move
people-title = Match people to their profiles
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
before-title = Before you leave this laptop
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
before-keys-title = Licence keys found on this laptop
before-keys-body = These apps keep their licence key on this laptop. Write each key down or keep it somewhere safe before you leave this laptop. PCTwin does not put keys in the move or its report.
before-keys-show = Show key
before-keys-show-label = Show key for { $app }
before-keys-hide = Hide key
before-keys-gone = PCTwin could not read this key again. Open { $app } to find it.

## S11 Get help with this task (a short problem code)

help-eyebrow = Help and privacy
help-title = Get help with this task
help-body = Share a short code that shows someone what went wrong. Check what it includes first.
help-making = Making a code…
help-code-title = Problem code
# $code is the code itself, for example "PCT-7F3K-21". Keep it unchanged.
help-code =
    .aria-label = Problem code { $code }
help-contains-title = What it includes
help-contains-problem = The kind of problem and how far the task got.
help-contains-step = The step of the move where it happened.
help-contains-versions = The PCTwin version, and which system this laptop runs, such as Windows or macOS.
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
help-home-eyebrow = Help
help-home-title = Common problems and what to do
help-home-body = Short guides for the things that most often get in the way. They work without the internet.
help-home-more = More help on getpctwin.com
help-guide-find-title = The laptops can’t find each other
help-guide-find-body = Put both laptops on the same Wi-Fi, not a guest network. Turn off any VPN while you pair. If it still does not work, switch on your phone’s hotspot and join both laptops to it.
help-guide-mac-network-title = Your Mac asks about the local network
help-guide-mac-network-body = Choose Allow, so PCTwin can find your other laptop. If you chose Don’t Allow, open System Settings, then Privacy & Security, then Local Network, and switch on PCTwin.
help-guide-firewall-title = Windows asks about the firewall
help-guide-firewall-body = Allow PCTwin on private networks, such as your home Wi-Fi. PCTwin only talks to your other laptop, on your own network.
help-guide-stopped-title = A move stopped part way
help-guide-stopped-body = Nothing copied and checked is lost, and your old laptop was not changed. Pair the laptops again: files already on the new laptop are found and not copied twice.
help-guide-others-title = Moving other people’s things
help-guide-others-body = Tick them on Who’s moving? on the old laptop. An administrator approves it once on each laptop. You never see their files. A new account’s temporary password is shown once, in the laptop’s own window; they choose their own at their first sign-in.
help-guide-code-title = Asking someone for help
help-guide-code-body = On any problem page, choose Get help. PCTwin makes a short code that says what went wrong, with no names, files or passwords in it, for you to send by WhatsApp or any app.

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
scan-title = Finding your files
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
overview-title = What PCTwin found
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
files-title = Choose the folders to move
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
dest-title = Where your folders go
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
plan-title = Check and approve the plan
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
plan-group-move = Move { $folder }
plan-place-change = Choose another place
plan-place-usual = Use the usual place
plan-place-free = { $size } free there
plan-place-no-room = There isn't enough free space there. Choose another place, or untick this folder.
plan-place-no-safe-name = PCTwin can't safely save files on that drive. Choose another place.
plan-place-cannot-open = PCTwin couldn't open that folder. Choose another place.
plan-place-locked = That drive is locked. Unlock it first, then choose it again.
plan-version = Plan version { $revision }
plan-details = More details
plan-preview-title = Before you start
plan-preview-fits = There is enough free space on this laptop.
plan-preview-fits-about = There should be enough free space on this laptop. This counts about { $extra } for the apps PCTwin installs and their data.
# Added 23 October 2026: what moves first, from the move's own order. $places is a list of folder names.
plan-preview-first = These move first, so you can start using your new laptop sooner: { $places }.
plan-preview-short = This laptop needs { $size } more free space. Free some space before you start, or the move stops when the drive is full.
plan-preview-short-about = This laptop needs about { $size } more free space, counting about { $extra } for the apps PCTwin installs and their data. Free some space before you start, or the move stops when the drive is full.
plan-preview-space-unknown = PCTwin could not read how much free space this laptop has.
plan-preview-synced = { $folder } is kept in { $service } on this laptop. Files moved there will upload to { $service } and use your space there.
plan-preview-time-one =
    { NUMBER($minutes) ->
        [one] About { $minutes } minute.
       *[other] About { $minutes } minutes.
    }
plan-preview-time = About { $low } to { $high } minutes.
plan-preview-renamed =
    { NUMBER($count) ->
        [one] { $count } file gets a slightly different name, because this laptop does not allow its name. The report lists it.
       *[other] { $count } files get a slightly different name, because this laptop does not allow their names. The report lists each one.
    }
plan-preview-too-big =
    { NUMBER($count) ->
        [one] { $count } file is too big for this laptop’s drive and will not be copied.
       *[other] { $count } files are too big for this laptop’s drive and will not be copied.
    }
plan-preview-refused =
    { NUMBER($count) ->
        [one] { $count } file has a name or folder this laptop does not allow, and will not be copied.
       *[other] { $count } files have a name or folder this laptop does not allow, and will not be copied.
    }
plan-preview-runs-at-sign-in =
    { NUMBER($count) ->
        [one] { $count } file would start by itself when someone signs in on this laptop. Check that you expect it.
       *[other] { $count } files would start by themselves when someone signs in on this laptop. Check that you expect them.
    }
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
plan-hand-too-many-files =
    { NUMBER($count) ->
        [one] { $count } folder holds more files than one move can carry, so it is not moving. After this move, send it on its own with Quick Send, or copy it with a USB drive.
       *[other] { $count } folders hold more files than one move can carry, so they are not moving. After this move, send each on its own with Quick Send, or copy them with a USB drive.
    }
plan-blocked-someone-unmapped = Someone on the old laptop has no account chosen on the new one yet. Choose where each person's files go, then come back.
plan-blocked-account-used-twice = Two people would end up in one account on the new laptop. Give each person their own.
plan-blocked-invalid = This plan cannot be approved as it is. Go back and check what you chose.
plan-blocked-fix = Choose accounts

## Tray menu

tray-show = Show PCTwin
tray-send = Send files
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
soon-move-title = Full move
soon-move-body = Move everything from your old laptop to a new one: your files, apps and settings. This is coming in a later version. For now, use Quick Send to send files.

## X01 Move progress, compact

progress-eyebrow = Your move
progress-title = Moving your things to this laptop
progress-title-old = Sending your things to your new laptop
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
progress-low-battery = This laptop is low on battery. Plug it in so the move is not interrupted.
progress-battery-paused = This laptop’s battery is almost empty, so the move is paused safely. Plug it in, then carry on. Nothing is lost.
progress-old-safe = Your old laptop stays unchanged.
progress-pause = Pause move
progress-compact = Show less
progress-expand = Show every item
progress-stop = Stop the move

## X02 Move progress, expanded

progress-all-title = Every item in the move
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
# Added 23 October 2026: T14, one tip under the progress bar, about every 12 seconds.
tips-label = Tip
tips-pause = Pause tips
tips-resume = Show more tips
tips-next = Next tip
tip-battery = This laptop is on battery. Plug it in if you can, so a long move is not cut short.
tip-closer = The connection is slow. Moving the laptops closer to the Wi-Fi router can speed it up.
tip-lids = Keep this laptop’s lid open. A closed lid can put it to sleep, which pauses the move.
tip-keep-using = You can keep using this laptop. PCTwin only reads from it.
tip-pause = You can pause the move at any time and carry on later.
tip-power-cut = If the power goes off, turn both laptops on and open PCTwin. It carries on from where it stopped, and nothing copied and checked is lost.

## F09 Mini panel

mini-title = Move in progress
mini-title-paused = Move paused
mini-what = moving your selected files
mini-progress =
    .aria-label = Move progress
mini-pause = Pause
mini-resume = Resume
mini-open = Open move
# Used when lane E reuses this panel for a Quick Send.
mini-title-send = Sending files
mini-what-send = sending selected files
mini-open-send = Open send
mini-progress-send =
    .aria-label = Send progress

## X05 Paused at a safe point

pause-title = The move is paused
pause-title-finishing = Pausing the move…
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
pause-resume = Resume the move

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
cancel-keep = Continue the move

## R01 The move is finished (outcome)

outcome-eyebrow = Move results
outcome-title = The move is finished
outcome-body = Here is what happened to everything you approved.
outcome-title-old = The move is finished
outcome-body-old = Here is what happened to everything you sent. This laptop stays as it was.
outcome-files-title = Selected files
outcome-status-partial = Partly done
# Added 23 October 2026: the after-move check on R01, counted the way the scan counts.
outcome-kinds-title = Checked after the move
outcome-kind-photos = Photos
outcome-kind-videos = Videos
outcome-kind-music = Music
outcome-kind-documents = Documents
outcome-kind-other = Other files
outcome-kind-count = { $copied } of { $planned } copied and checked
outcome-next-title = What to do next
outcome-old-safe = Anything not finished stays listed here. Nothing was removed from your old laptop.
outcome-landed-title = Where your things are now
outcome-report = View complete report
outcome-home = Return home
outcome-person-copied =
    { NUMBER($count) ->
        [one] { $count } file copied.
       *[other] { $count } files copied.
    }
outcome-person-not-done =
    { NUMBER($count) ->
        [one] { $count } did not finish.
       *[other] { $count } did not finish.
    }
outcome-person-at-sign-in = They are in place when { $name } next signs in on this laptop.
move-fallback-folder = From your old laptop
move-stop = Stop the move
move-before-open = While you wait: before you leave this laptop
move-get-help = Get help
pickup-eyebrow = Your move
pickup-title = Tidy up the stopped move
pickup-body = A move on this laptop stopped part way, perhaps because the laptop restarted or lost power. Tidy up first, then pair the laptops again to carry on.
pickup-unfinished-title = Unfinished copies
pickup-unfinished-body =
    { NUMBER($count) ->
        [one] { $count } file was being copied when the move stopped.
       *[other] { $count } files were being copied when the move stopped.
    }
pickup-unfinished-badge = To tidy
pickup-tidy = Tidy up
pickup-tidying = Tidying up. Finished copies are checked and put in place; nothing of yours is changed.
pickup-done = Tidying up is finished. Pair the laptops again to continue: what is already here is found and not copied twice.
pickup-failed = Tidying up did not work this time. Nothing of yours was changed. Try again, or close PCTwin on this laptop and open it again first.
pickup-put-in-place = put in place
pickup-kept = kept to carry on
pickup-not-finished =
    { NUMBER($count) ->
        [one] { $count } file could not be finished. The next move copies it again.
       *[other] { $count } files could not be finished. The next move copies them again.
    }
pickup-left-for-later =
    { NUMBER($count) ->
        [one] { $count } file is on a drive that is not connected now. Connect it and tidy up again.
       *[other] { $count } files are on a drive that is not connected now. Connect it and tidy up again.
    }
pickup-later = Not now
ready-eyebrow = From your old laptop
ready-title = Your things are in your folders
ready-body = Your things from your old laptop were moved to this one, and are now in your own folders.
ready-landed = files in your folders
ready-where-title = Where they are
ready-where-body = In Documents, Pictures, Music, Videos and your other usual folders, as on your old laptop.
ready-kept =
    { NUMBER($count) ->
        [one] { $count } file was kept aside, because a file of that name was already here. Both are kept.
       *[other] { $count } files were kept aside, because files of those names were already here. Both are kept.
    }
ready-failed =
    { NUMBER($count) ->
        [one] { $count } file could not be put in your folders. Ask the person who did the move to move it again.
       *[other] { $count } files could not be put in your folders. Ask the person who did the move to move them again.
    }
ready-done = Done
move-status-kept = Kept
move-wait-eyebrow = Your move
move-wait-scanning-title = Finding your files
move-wait-scanning-body = Your old laptop is looking through your folders. It only reads them: nothing is opened, changed or removed.
move-found = Found so far: { $size } of your things.
move-wait-approval-title = Waiting for your new laptop
move-wait-approval-body = Your files are ready to go. Check the plan on your new laptop and approve it to start.
move-wait-list-title = Waiting for your old laptop
move-wait-list-body = Your old laptop is finding your things. Then what moves is chosen there. When that is done, the list appears here for you to accept. This can take a few minutes.
move-wait-starting-title = Starting the move
move-wait-starting-body = The plan is approved. Copying starts in a moment.
move-wait-admin-title = Waiting for an administrator
move-wait-admin-body = This laptop is asking for an administrator’s approval to move the other people’s things too. Answer the window your laptop opened.
move-others-not-approved = The administrator did not approve it, so only your things move this time. The other people’s things stay where they are.
move-others-failed = PCTwin could not start what it needs to move the other people’s things, so only your things move this time. Nothing of theirs was changed.
move-others-not-here = Moving other people’s things from this laptop is not ready yet in this version, so only your things move.
move-others-allow-in-settings = First, an administrator needs to allow PCTwin in System Settings, Login Items. That page is open now. Only your things move this time; once PCTwin is allowed there, your next move can take the other people’s things too.
approve-prompt = PCTwin will read the files of { $names } to move them to the new laptop. An administrator of this laptop approves this move only.
who-here-title = Who’s moving?
who-here-body = Your things always move. Tick anyone else on this laptop whose things should move too. PCTwin never shows you their files.
who-here-you = { $name } (you)
who-here-you-hint = Always moving
who-here-approval = Moving someone else’s things needs an administrator’s approval, once, for everyone you tick. Your laptop will ask in its own window.
who-here-not-here = Moving other people’s things from this laptop is not ready yet in this version. Only your things move.
place-title = Where do their things go?
place-body = Other people’s things came from the old laptop too. Choose an account on this laptop for each person, a new account for them, or leave them out. You never see their files.
place-size =
    { NUMBER($count) ->
        [one] { $count } file, { $size }
       *[other] { $count } files, { $size }
    }
place-select = Where { $name }’s things go
place-note = New accounts, and keeping their things safe until they sign in, need an administrator’s approval once, after you approve the plan. Each new account’s temporary password is shown once, in your laptop’s own window.
place-todo =
    { NUMBER($count) ->
        [one] Choose where { $count } more person’s things go.
       *[other] Choose where { $count } more people’s things go.
    }
fda-title = PCTwin needs permission to read some folders
fda-body =
    { NUMBER($count) ->
        [one] { $count } place could not be read because of your Mac’s privacy settings. Give PCTwin access, then check again, so none of your files are missed.
       *[other] { $count } places could not be read because of your Mac’s privacy settings. Give PCTwin access, then check again, so none of your files are missed.
    }
fda-step-1 = Open System Settings, then Privacy & Security.
fda-step-2 = Choose Full Disk Access.
fda-step-3 = Switch on PCTwin. Your Mac may ask for your password.
fda-open = Open these settings
fda-note = PCTwin only reads your folders to copy them. It never changes anything on this laptop.
fda-again = Check again
fda-without = Continue without them
password-title = PCTwin: a new account
password-body =
    { $name } now has an account on this laptop.
    
    Sign-in name: { $account }
    Temporary password: { $password }
    
    Give these to { $name }. At their first sign-in they choose their own password. Write it down now: PCTwin does not keep it.
move-ended-eyebrow = Your move
move-ended-kept-title = Already copied
move-ended-kept-body = Everything copied and checked before this happened stays on the new laptop.
move-ended-old-title = Your old laptop
move-ended-old-body = Nothing on it was changed or removed.
move-ended-old-badge = Unchanged
move-ended-next = To try again, pair the laptops again. Files already copied and checked are found and not copied twice.
move-lost-title = The connection to your other laptop was lost
move-lost-body = The laptops stopped reaching each other, so the move could not go on. Check that both are switched on, awake and near each other.
move-not-approved-title = The move did not start
move-not-approved-old = The plan was not approved on the new laptop, so nothing was copied.
move-not-approved-new = You did not approve the plan, so nothing was copied.
move-failed-title = The move stopped because of an error
move-failed-body = The move could not go on. If it happens again, More details below says what went wrong.
move-peer-system-windows = Windows
move-peer-system-mac = Mac
move-peer-system-linux = Linux
move-peer-system-other = Another system
move-update-here-title = Update PCTwin on this laptop
move-update-here-body = This laptop has an older PCTwin than the other laptop, so they can't move files together. Update PCTwin here, then start again. Nothing was copied.
move-update-there-title = Update PCTwin on the other laptop
move-update-there-body = The other laptop has an older PCTwin than this one, so they can't move files together. Update PCTwin there, then start again. Nothing was copied.
move-refused-title = The move could not start
move-refused-not-paired = Your laptops are no longer paired. Pair them again to start the move.
move-refused-busy = A move is already running on this laptop.
move-refused-not-moving = There is no move running on this laptop.
move-refused-plan-changed = The plan changed before it was approved. Pair the laptops again to see the new plan.
move-refused-failed = This laptop’s folders could not be made ready for the move. Pair the laptops again to try once more.
drive-problem-title = The move needs your attention
drive-problem-body = Copies saved in { $place } did not match the originals when PCTwin checked them, several times. The drive may be failing, full or unplugged, so copying there is paused.
drive-problem-kept-title = Already copied and checked
drive-problem-waiting-title = Still to copy to { $place }
drive-problem-badge = Paused
drive-problem-fix = Check that the drive is plugged in and has free space, then continue. Nothing you chose is left out to get past this. You can also stop the move.
drive-problem-carry-on = Continue

## R02 Your move, item by item (detailed report)

report-title = What moved
report-body = Select a group to see what is in it. Things that need you are at the top.
report-needs-title = { $count ->
        [one] Needs you: 1
       *[other] Needs you: { $count }
    }
report-group-photos = Photos
report-group-videos = Videos
report-group-music = Music
report-group-documents = Documents
report-group-other = Other files
report-group-size = { $files ->
        [one] 1 file
       *[other] { $files } files
    } · { $size }
report-group-done = All copied and checked
report-group-some = { $done } of { $total } copied and checked
report-group-left = { $count ->
        [one] 1 not moved: you left it out
       *[other] { $count } not moved: you left them out
    }
report-filter-label = Show
report-col-item = Item
report-col-result = Outcome
report-col-details = Check or next step
report-from = From: { $path }
report-to = To: { $path }
report-none = No items have this result.
report-more-details = More details
# Added 23 October 2026: what the report says beside a line's result (changed since the scan).
report-note-changed-since-scan = Changed since the scan: it was copied as it is now.
report-note-no-longer-there = No longer there: it was moved, renamed or deleted on the old laptop before it could be sent.
report-note-time-not-kept = Date not kept: this drive cannot store the date it was last changed, so it shows the date it was copied.
report-note-no-permission = Not added: this folder is shared by everyone on this laptop, and only an administrator can add to it. Ask whoever looks after this laptop, or copy it there yourself.

# Evidence: how a copied item was checked.
evidence-read-back = Checked by reading it back from the drive.
evidence-memory = Checked from this laptop’s memory, not the drive.
evidence-already-there = Already on the new laptop: the same file was there, so it was not copied again.
evidence-recovered = Checked before the laptop restarted, and checked again after.
evidence-new-laptop = Checked by the new laptop.
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
reason-drive-locked = The drive this goes to is locked. Unlock it (for example with its BitLocker password or recovery key), then move it again.
reason-sign-in = It is in place, but you need to sign in. Open it and sign in.
reason-activation = The app is in place, but it needs its licence on the new laptop. Open it and activate it with your licence or account.
reason-permission = The new laptop needs to give permission first. Open the item and choose Allow when asked.
reason-online-only = This file is stored online only, so it was not on the old laptop to copy. Download it on the new laptop from the app that stores it online.
reason-manual-install = This app must come from its maker’s website. Download and install it there.
reason-restart = The new laptop must restart to finish. Restart it when you are ready.
reason-finish-by-hand = PCTwin cannot do this step for you. Follow the steps shown for it in this report.
reason-parts-missing = Some files inside this folder did not arrive, so it is not complete on the new laptop. They are listed in this report with why. Send them again, or copy the folder another way.
reason-moved-file-missing = This setting points at a file or folder that is not on the new laptop under the name the move gave it, so PCTwin left the setting as it was. Open the app and choose the file again.

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
mini-title-send-paused = Send paused
cancel-ack-hint = Tick the box above to turn on the stop button.
progress-state-placing = Putting in place
progress-state-existing = Already there
outcome-stopped-title = Your move was stopped
outcome-stopped-body = You stopped the move. What was copied and checked before then is kept. The rest was not copied.
outcome-broke-title = Your move ended early
outcome-broke-body = Something went wrong between the laptops, so the move could not go on. What was copied and checked before then is kept. The rest was not copied.
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
qs-eyebrow-receiving = Receiving on your laptop
qs-label-this = This laptop
qs-label-other = The other laptop
qs-pick-failed = The picker could not open. Try again.
qs-handed =
    { NUMBER($count) ->
        [one] 1 file or folder was handed to PCTwin to send. Check the list before you continue.
       *[other] { $count } files or folders were handed to PCTwin to send. Check the list before you continue.
    }
qs-pick-too-many = That is more files than one send can carry. Untick some folders, or send them in parts.
qs-problem-lost-title = The connection between the laptops was lost
qs-problem-failed-title = The files could not be sent this time
qs-problem-body = Anything already received and checked stays where it is. Pair the laptops again to try once more.
# The route between the two laptops, for example "Blue Fox" to "Green Owl".
qs-route =
    .aria-label = From { $from } to { $to }
qs-route-sender = Sending
qs-route-receiver = Receiving · same Wi-Fi
qs-tally =
    { NUMBER($count) ->
        [one] { $count } selected item · { $size }
       *[other] { $count } selected items · { $size }
    }
qs-tally-badge = Nearby send

## Q01 Quick Send home

qs-home-title = Send files to a laptop nearby
qs-home-body = Or receive files from one.
qs-send-title = Send files
qs-send-body = Choose files or folders on your laptop.
qs-receive-title = Receive files
qs-receive-body = Let a laptop nearby send files to yours.
qs-home-action = Choose files
qs-receiving = Ready to receive. Other laptops nearby see yours as { $label }.
qs-receive-stop = Stop receiving
qs-no-nearby = No laptops found nearby. Open PCTwin on the sending laptop and keep both on the same Wi-Fi.

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
qs-online-only = Some files are only online. Make them available on your laptop first, then send them.
qs-protected = PCTwin cannot read { $name }. Allow access, or remove it from the list.
qs-allow = Allow access
qs-choose-continue = Continue

## Q12 Send progress

qs-sending-title = Sending your files
qs-sending-body = Copies go straight to the receiving laptop.
qs-sending-progress =
    .aria-label = Sending files
qs-sending-current = Now sending: { $name }
# $speed is measured and already formatted, for example "3.1 MB/s".
qs-sending-speed = { $speed }, measured
qs-step-copying = Copying
qs-step-copying-body =
    { NUMBER($done) ->
        [one] { $done } file sent.
       *[other] { $done } files sent.
    }
qs-step-checking = Checking
qs-step-checking-body = Each file is compared with the original.
qs-sending-paused = Paused. Files already received and checked are kept.
qs-sending-disconnected = The connection to { $label } dropped. Files already checked are kept. Trying again by itself…
qs-sending-needs-action =
    { NUMBER($count) ->
        [one] { $count } item needs your action.
       *[other] { $count } items need your action.
    }
qs-pause = Pause send
qs-resume = Resume
qs-compact = Show less
qs-stop = Stop this send
qs-sending-complete-title = The send is complete

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
qs-stop-keep = Keep sending

## F08 Your moves, kept on this laptop (history)

history-title = Your move history
history-body = Open a report again, or carry on with a saved move.
history-row-move = { $from } → { $to }
history-status-finished = Completed
history-status-steps-left =
    { NUMBER($count) ->
        [one] Completed, { $count } step for you
       *[other] Completed, { $count } steps for you
    }
history-status-paused = Paused
history-status-stopped = Stopped
history-open-report = Report
history-undo = Undo
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
qs-sending-card = Sending files
qs-progress-of = of { $total }
history-open-name = { $what }: { $title }

## R05 Undo: what undo will do on each drive, said before the person starts, and how a copy ended

undo-here-removes = Undo removes the copies PCTwin made that you haven't changed. Anything you changed is kept.
undo-here-to-trash = Undo moves the copies PCTwin made that you haven't changed to the Trash. Anything you changed is kept, and nothing is deleted.
undo-here-keeps-fat = Undo will keep every copy on { $drive }. This drive uses FAT or exFAT, which reuses the numbers that tell files apart, so PCTwin can't be sure which files are its own. You can delete them yourself.
undo-here-keeps-other = Undo will keep every copy on { $drive }. PCTwin can't move or remove files safely on this kind of drive. You can delete them yourself.
undo-here-more = More details
undo-here-more-trash = A Mac can't show PCTwin that no other app is still writing to a file, so PCTwin never deletes one. It moves each copy into the Trash on the same drive. You can put it back from the Trash.
undo-here-more-fat = On FAT and exFAT drives, a new file can get the number a removed file had. PCTwin only acts on files it can prove are its own and unchanged, so it keeps these.
undo-done-trashed = Moved to the Trash as “{ $name }”. You can put it back from the Trash.
undo-kept-fat = Kept. This drive (FAT or exFAT) reuses the numbers that tell files apart, so PCTwin can't be sure this is its copy. If you don't need it, you can delete it yourself.
undo-kept-no-trash = Kept. PCTwin couldn't put this copy in the Trash on its drive. If you don't need it, you can delete it yourself.

## Quick Send (26 October 2026): pairing in Quick Send's words, where received files go, and the Done page

qs-pair-old-eyebrow = Sending
qs-pair-old-body = On the receiving laptop, choose Receive files, then enter this code.
qs-pair-old-waiting = Waiting for the receiving laptop…
qs-pair-old-label-hint = The receiving laptop will find yours by this name.
qs-pair-old-steps-title = On the receiving laptop
qs-pair-old-step-1 = Open PCTwin, choose Quick Send, then Receive files.
qs-pair-old-left = The receiving laptop stopped pairing before a number was chosen, so nothing was shared. Here is a new code.
qs-pair-network-hint = Choose the network the receiving laptop is on.
qs-pair-choose-body = Look at the receiving laptop. Choose the number it shows.
qs-pair-choose-renewed = New numbers are shown. Choose the one on the receiving laptop.
qs-pair-choose-note = If the receiving laptop shows no number, or none of these match it, choose None of these match.
qs-pair-heads-up-firewall = Windows may ask whether PCTwin can accept connections. Choose Allow, so the receiving laptop can reach yours.
qs-pair-new-eyebrow = Receiving
qs-pair-new-searching-title = Looking for the sending laptop…
qs-pair-new-searching-body = Keep PCTwin open on the sending laptop, on the same Wi-Fi as yours.
qs-pair-new-search-failed-title = Could not look for the sending laptop
qs-pair-new-list-title = Which laptop is sending?
qs-pair-new-list-body = Choose the name the sending laptop shows on its screen.
qs-pair-new-same-label = Another laptop nearby has this name. Check the sending laptop’s screen.
qs-pair-new-none-title = No sending laptop found
qs-pair-new-none-body = Check that PCTwin is open on the sending laptop and showing a code, and that both laptops are on the same Wi-Fi. Then search again.
qs-pair-new-code-hint = Read it from the screen on { $label }. It changes every minute.
qs-pair-new-number-title = Choose this number on { $label }
qs-pair-new-number-body = Three numbers are shown on { $label }. Choose this one there.
qs-pair-new-number-waiting = Waiting for { $label }…
qs-pairing-code-invalid = That code needs six digits. Check the screen on { $label } and type it again.
qs-pairing-code-wrong = That code did not match. Check the screen on { $label } and try again.
qs-pairing-code-expired = That code has changed. Type the new one from the screen on { $label }.
qs-pairing-busy = Pairing with { $label } cannot start now: it is pairing with another laptop. Wait a moment, then try again.
qs-pairing-wait = A new code is being made on { $label }. Wait for it, then type the new code.
qs-pairing-locked = Pairing stopped after too many wrong tries. On { $label }, choose Start again.
qs-pairing-unreachable = Your laptop could not reach { $label }. Check that both are on the same Wi-Fi, then try again.
qs-pairing-wrong-number = That number did not match the receiving laptop’s. Nothing was shared. Type the new code on the receiving laptop to try again.
qs-pairing-no-match = The numbers did not match on { $label }. Nothing was shared. Type the new code from its screen.
qs-pairing-timed-out = No number was chosen in time. Try again with the new code on { $label }.
qs-pairing-connection-lost-new = The connection to { $label } dropped. Nothing was shared.
qs-pairing-choose-network = Your laptop is on more than one network. Choose the one the receiving laptop is on:
qs-pairing-failed-new = Pairing did not work this time. Nothing was shared. Type the new code from the screen on { $label }.
qs-folder-line = Files will be saved in { $folder }.
qs-folder-change = Change
qs-folder-title = Where should the files go?
qs-folder-none = PCTwin cannot use the folder for received files. Choose a folder for the files, then connect.
qs-folder-choose = Choose a folder
qs-folder-failed = PCTwin cannot save files in that folder. Choose another one.
qs-done-sent-title = Your files were sent
qs-done-here-title = The files were received
qs-done-partial-title = Some files did not arrive
qs-done-stopped-title = This send was stopped
qs-done-empty-title = Nothing was sent
qs-done-arrived =
    { NUMBER($count) ->
        [one] { $count } file · { $size } arrived and matched the original
       *[other] { $count } files · { $size } arrived and matched the originals
    }
qs-done-arrived-count =
    { NUMBER($count) ->
        [one] { $count } file arrived and matched the original
       *[other] { $count } files arrived and matched the originals
    }
qs-done-already =
    { NUMBER($count) ->
        [one] { $count } file was already there, identical, so it was not sent again.
       *[other] { $count } files were already there, identical, so they were not sent again.
    }
qs-done-stopped-body = Files that arrived before the stop were checked and stay where they were saved. Nothing else was sent.
qs-done-folder = Saved in { $folder }
qs-done-open-folder = Open folder
qs-done-dates-new =
    { NUMBER($count) ->
        [one] This file shows the time it arrived, so it is at the top of the folder.
       *[other] These { $count } files show the time they arrived, so they are at the top of the folder.
    }
qs-done-keep-dates = Keep original dates
qs-done-dates-kept =
    { NUMBER($count) ->
        [one] { $count } file has its original date again.
       *[other] { $count } files have their original dates again.
    }
qs-done-dates-failed = The original dates could not be put back. The files are fine and stay where they are.
qs-done-open-failed = The folder could not be opened. It may have been moved or deleted.
qs-done-not-opened = PCTwin did not open or run any received file.
qs-done-problems-title = Files that need attention
qs-done-not-arrived = It did not arrive.
qs-done-not-completed = It was not sent before the stop.
qs-done-needs-action = It is here, but not finished. What to do is under More details.
qs-done-mark-not-kept = It is here, but this drive could not mark it as downloaded from another laptop. Open it with care.
qs-done-see-files = See the files
qs-done-renamed = Saved as { $name }, because a file with that name was already there.
qs-done-identical = An identical file was already there, so it was not sent again.
qs-done-send-missing = Send the missing files
qs-done-send-missing-body = Only the files that did not arrive. You pair the laptops again first.
qs-done-done = Done
qs-stop-now = Stop sending
qs-stop-finished = This send has already finished.
qs-route-receiving = Receiving
qs-receiving-title = Receiving files
qs-receiving-body = Copies come straight from { $label }.
qs-receiving-card = Receiving files
qs-receiving-progress =
    .aria-label = Receiving files
qs-receiving-current = Now receiving: { $name }
qs-step-receiving-body =
    { NUMBER($done) ->
        [one] { $done } file arrived.
       *[other] { $done } files arrived.
    }
qs-tally-files =
    { NUMBER($count) ->
        [one] { $count } file · { $size }
       *[other] { $count } files · { $size }
    }
qs-receive-pause = Pause
qs-label-yours = Your laptop
qs-label-receiving = Receiving laptop
qs-label-sending = Sending laptop
qs-peer-receiving = the receiving laptop
qs-peer-sending = the sending laptop
qs-pair-old-label-title = Your laptop’s name
qs-pair-old-rename-label = Name for your laptop
qs-pair-old-step-2 = If a list appears, choose your laptop’s name.
qs-pair-old-locked-body = After five wrong codes, pairing stopped to keep your laptop safe. Nothing was shared. If it was you, choose Start again for a new code.
qs-pair-new-needs-update = It has an older PCTwin. Update PCTwin on it, then search again.
qs-pairing-no-network = Your laptop is not on a Wi-Fi or wired network. Connect it, then try again.
qs-problem-room-title = Not enough room in { $folder }
qs-problem-room-body = These files need { $needed }, and { $room } can be used there. Nothing was saved. Free up space or choose another folder, then try again.
qs-done-refused-title = The receiving laptop did not take these files
qs-done-refused-body = It may not have enough room. Nothing was sent.

## Phase 2 screens (Task List 2.8): shared parts

guard-state-not-started = Not started yet
guard-state-preparing = Getting ready
guard-state-awaiting-person = Waiting for your choice
guard-state-awaiting-system = Waiting for your system’s approval
guard-state-awaiting-maker = Waiting for the app’s maker or online service
guard-state-working = Working
guard-state-checking = Checking what was done
guard-state-applying = Putting things in place
guard-state-pause-requested = Pausing after the current step. Not paused yet.
guard-state-paused = Paused
guard-state-source-unavailable = The old laptop or drive is not connected
guard-state-blocked = Something needs fixing first
guard-state-partial = Partly done
guard-state-retrying = Trying again
guard-state-reconciling = Checking what was finished
guard-state-failed = This did not work
guard-state-complete = Done
guard-state-cancelled = Stopped. What was already done is kept.
guard-refused-unavailable = PCTwin could not check this right now. Nothing was done. Try again in a moment.
guard-refused-wrong-job = This was for another move. Nothing was done.
guard-refused-old-session = This screen was out of date, so nothing was done. Here is what is true now.
guard-refused-plan-changed = The plan changed while you were looking, so nothing was done. Look at it again.
guard-refused-not-now = That cannot be done at this step. Nothing was done.
guard-refused-not-shown = That choice is not on this screen any more. Nothing was done.
guard-progress-label = Progress
guard-progress-mark = { $percent }% done
guard-progress-of = { $done } of { $total }
guard-progress-unknown-total = { $done } so far. The total is not known yet.
guard-progress-none = Nothing measured yet.
guard-and-more =
    { $count ->
        [one] And { $count } more, listed in the report.
       *[other] And { $count } more, listed in the report.
    }
guard-not-known-yet = Not known yet
guard-free = { $free } free
guard-free-unknown = Free space not known
guard-items-size = { $items } items, { $size }
guard-try-again = Try again
guard-pause = Pause
guard-resume = Resume
guard-check-again = Check again
guard-strip-plan = Your plan
guard-strip-report = Your report
guard-strip-permission = Approval
guard-strip-rescue = Rescue
guard-strip-undo = Undo
guard-strip-checking = Checking

## X09 checking after a restart, and status unavailable

guard-checking-title = Checking what was finished
guard-checking-body = PCTwin is looking at what was finished before it closed. Your choices come back once it knows.
guard-unavailable-title = Status unavailable
guard-unavailable-body = PCTwin cannot say right now what this move is doing. Nothing is shown as done until it can. Check again in a moment.

## 2.9 What the move will change

guard-changes-title = What the move will change
guard-changes-body = This is everything the move will change on this laptop, and what undo can do for each. Nothing has changed yet.
guard-changes-download = Download from the internet
guard-changes-download-none = None
guard-changes-permissions = System approval
guard-changes-one-prompt = Your system asks once
guard-changes-no-prompt = Not needed
guard-changes-restore-point = Windows restore point
guard-changes-restore-point-first = Asked for first
guard-changes-restore-point-none = Not used
guard-changes-install =
    { $count ->
        [one] To install: { $count } app
       *[other] To install: { $count } apps
    }
guard-changes-merge =
    { $count ->
        [one] Added beside what is here: { $count } item
       *[other] Added beside what is here: { $count } items
    }
guard-changes-replace =
    { $count ->
        [one] To replace, keeping the old version: { $count } item
       *[other] To replace, keeping the old versions: { $count } items
    }
guard-changes-leave-alone =
    { $count ->
        [one] Left as it is: { $count } item
       *[other] Left as they are: { $count } items
    }
guard-changes-space = Room on each drive at the busiest moment
guard-changes-why-prompt = Why your system will ask
guard-changes-approve = Approve these changes
guard-undo-legend-can = PCTwin can undo it.
guard-undo-legend-keeps = PCTwin keeps it. Undo leaves it alone.
guard-undo-legend-you = Needs you. Only its service or maker can undo it.
guard-undo-can-reverse = PCTwin can undo it
guard-undo-can-remove-app = PCTwin can remove the app. It does not touch files you made.
guard-undo-must-keep = Kept: it was already here
guard-undo-needs-you-service = Needs you: only the online service can undo it
guard-undo-needs-you-maker = Needs you: sign-ins and licences are the maker’s
guard-drive-needs = Needs { $needed }. { $free } free.
guard-drive-needs-free-unknown = Needs { $needed }. The drive does not say how much is free.
guard-drive-fits = Fits
guard-drive-short = Short by { $short }
guard-drive-unknown = Not known
guard-permission-read-other-people = To read the files of other people on the old laptop
guard-permission-make-accounts = To make their accounts on this laptop
guard-permission-shared-folders = To put things in shared folders your account cannot change
guard-permission-install-for-everyone = To install apps for everyone on this laptop
guard-permission-restore-point = To make a Windows restore point first

## X06 and H10 something changed after approval

guard-changed-title =
    { $count ->
        [one] { $count } thing changed after you approved
       *[other] { $count } things changed after you approved
    }
guard-changed-body = Only these need checking again. Everything else stays approved and continues.
guard-changed-changed-on-old-laptop = It changed on the old laptop.
guard-changed-app-updated = The app updated itself.
guard-changed-destination-changed = The place it goes to changed.
guard-changed-new-scope = It is new: another person or laptop joined the move.
guard-changed-permission-ended = The approval for it has ended.
guard-changed-leave-out = Leave these out
guard-changed-approve = Approve these again

## M31, M05 space on the new laptop

guard-space-title = Space on the new laptop
guard-space-title-short = The new laptop is short of space
guard-space-body = PCTwin counted everything that lands on each drive, including room for undo. Nothing you chose is dropped to make it fit.
guard-space-proposal = What PCTwin suggests for what does not fit
guard-keep-off-leave-in-cloud = Left online, where they are stored now
guard-keep-off-offload-drive = Put on your other drive
guard-keep-off-leave-on-old-laptop = Left on the old laptop. Keep it until these are moved.
guard-offload-title = Your other drive
guard-offload-connected = Connected
guard-offload-not-connected = Not connected
guard-offload-approved = You chose this drive. Files on it are not on the new laptop: connect it to open them.
guard-offload-ask = PCTwin can put some things on this drive. It needs your approval for this drive separately.
guard-offload-use = Use this drive
guard-space-approve = Approve this plan
guard-safe-in = Stored in { $provider }
guard-safe-in-cloud = Stored online
guard-safe-in-detail =
    { $count ->
        [one] { $count } file is stored online only. It comes back when you sign in on the new laptop.
       *[other] { $count } files are stored online only. They come back when you sign in on the new laptop.
    }
guard-your-cloud = your online storage

## M09 keeping a copy of online-only files

guard-local-title = Keep a copy of your online-only files?
guard-local-body =
    { $count ->
        [one] { $count } file in { $provider } is stored online only. You can keep a copy of it on the new laptop too.
       *[other] { $count } files in { $provider } are stored online only. You can keep a copy of them on the new laptop too.
    }
guard-local-download = To download on the old laptop
guard-local-old-space = Room on the old laptop
guard-local-how = PCTwin downloads them a few at a time on the old laptop, sends them, then sets them back to online only there.
guard-local-paused-metered = Paused: this connection may charge for data. It carries on by itself on a free connection, or now if you allow it.
guard-local-paused-no-space = Paused: the old laptop is out of room for the next few files. The rest stay safe in { $provider }.
guard-local-done = Done. The copies are on the new laptop.
guard-local-keep = Keep a copy
guard-local-leave = Leave them in { $provider }
guard-local-allow-metered = Allow on this connection

## M07 who goes where

guard-owners-title = Who goes where
guard-owners-body = Each person’s things go to their own account on this laptop. Matching names alone never decide it.
guard-owners-from = From { $laptop }
guard-owners-to-account = To the account { $account } on this laptop
guard-owners-new-account = To a new account on this laptop
guard-owners-own-folder = Their own “From { $laptop }” folders, because names clash
guard-owners-usual-folders = Their usual folders
guard-owners-approve = Approve

## X12 what did not finish

guard-report-title-partial = Some things did not move
guard-report-title-done = Everything in this part moved
guard-report-body-partial = Everything below is still on the old laptop. Try each again when you are ready. What already moved stays.
guard-report-body-done = Nothing is left to try again.
guard-line-app = The app { $name }
guard-line-items =
    { $count ->
        [one] { $count } item
       *[other] { $count } items
    }
guard-line-settings =
    { $count ->
        [one] { $count } setting
       *[other] { $count } settings
    }
guard-line-why-failed = From { $laptop }: it did not finish.
guard-line-why-laptop-not-connected = From { $laptop }: that laptop is not connected. Connect it to the same Wi-Fi and keep it awake.
guard-line-why-waits-for-another = From { $laptop }: it waits for another step that did not finish. Fix that one first.
guard-line-why-not-started = From { $laptop }: the move stopped before it started.

## M25, H10 the system’s approval, and the restore point

guard-permission-title-explain = PCTwin needs one approval from your system
guard-permission-title-waiting-for-system = Answer your system’s question
guard-permission-title-denied = You did not approve
guard-permission-title-closed-without-answer = The question was closed without an answer
guard-permission-title-given = Approved for this move
guard-permission-title-ended-by-sleep = The approval ended when the laptop went to sleep
guard-permission-title-ended-other-session = The approval ended when someone else signed in
guard-permission-body-explain = Your system asks once for the whole move. Only the steps listed below use it, and it ends with the move.
guard-permission-body-waiting-for-system = Your system’s own window is open. Type your password there if it asks.
guard-permission-body-denied = Only the steps that needed it are left out. Everything else carries on.
guard-permission-body-closed-without-answer = Closing the window does not count as approval, so nothing that needs it was done. Ask again, or continue without it.
guard-permission-body-given = PCTwin uses it only for the steps listed below, and it ends with this move.
guard-permission-body-ended-by-sleep = Nothing that needs it was done after that. Ask again to carry on.
guard-permission-body-ended-other-session = Nothing that needs it was done after that. Ask again when you are back on your account.
guard-permission-never-password = PCTwin never asks for your password and never sees it. Only your system does.
guard-permission-not-now = Continue without it
guard-permission-ask = Continue to your system
guard-permission-ask-again = Ask again
guard-restore-title = Windows restore point
guard-restore-made = Made at { $time }.
guard-restore-earlier-exists = Windows already has one from { $time }, so it did not make another.
guard-restore-not-chosen = Not made: you chose not to.
guard-restore-not-made-not-windows = Not made: restore points are only on Windows.
guard-restore-not-made-protection-off = Not made: System Protection is off for this drive. You can turn it on in System Properties.
guard-restore-not-made-needs-admin = Not made: it needs your system’s approval.
guard-restore-not-made-unconfirmed = Not made: Windows said yes, but no restore point can be found.
guard-restore-not-made-failed = Not made: Windows could not make one.

## T01 to T04 rescue from a laptop that will not start

guard-rescue-title-intro = Copy files from a laptop that will not start
guard-rescue-title-pick-drive = Choose the drive to rescue
guard-rescue-title-locked = This drive is locked
guard-rescue-title-no-system = This is not a laptop’s drive
guard-rescue-title-salvage = Here is what could be read
guard-rescue-body-intro = Take the drive out of the old laptop and connect it to this one with a drive adapter or enclosure.
guard-rescue-body-pick-drive = Choose the old laptop’s drive. The drive you copy to cannot be the one you rescue.
guard-rescue-body-locked = Unlock it in your system first. Then check again.
guard-rescue-body-no-system = This drive has no laptop system and home folders. If it is an ordinary USB drive, use it on the new laptop instead.
guard-rescue-body-salvage = Check the list, then use what could be read.
guard-rescue-can = What a rescue can do
guard-rescue-can-body = Read your files, and the people and settings found on the drive, from a drive that still works.
guard-rescue-cannot = What it cannot do
guard-rescue-cannot-body = Repair the drive, read parts that are damaged, or open a drive whose key you do not have.
guard-rescue-read-only-title = The drive is only read
guard-rescue-read-only-promise = PCTwin only reads it. Nothing on it is changed, repaired or formatted.
guard-rescue-system-read-only = Your system opened it read only, so nothing on it can change.
guard-rescue-pctwin-reads-only = PCTwin only read it. Your system had it open for changes, so other apps could still change it.
guard-rescue-refused-target = That drive is where things are put, so it cannot also be rescued. Choose another drive.
guard-rescue-drive-detail = { $size }, { $fs }
guard-rescue-drive-is-target = Things from the move go to this drive.
guard-rescue-drive-cannot = Cannot be rescued
guard-rescue-choose = Rescue this drive
guard-rescue-bit-locker-step-1 = Open the drive in File Explorer.
guard-rescue-bit-locker-step-2 = Type its recovery key when Windows asks. It is often saved in the owner’s Microsoft account, at aka.ms/myrecoverykey.
guard-rescue-bit-locker-step-3 = Come back here and choose Check again.
guard-rescue-file-vault-step-1 = Unlock it on a Mac with the owner’s login password or recovery key.
guard-rescue-file-vault-step-2 = Come back here and choose Check again.
guard-rescue-apple-chip-step-1 = This Mac keeps its drive locked to itself, so taking the drive out does not work.
guard-rescue-apple-chip-step-2 = If the Mac still starts in Recovery, use Share Disk with a cable. If it does not start at all, only an Apple repair centre can help.
guard-rescue-never-key = PCTwin never asks for the key and never keeps it.
guard-rescue-readable =
    { $count ->
        [one] { $n } item can be read.
       *[other] { $n } items can be read.
    }
guard-rescue-unreadable =
    { $count ->
        [one] { $count } item could not be read
       *[other] { $count } items could not be read
    }
guard-rescue-look = Look for the drive
guard-rescue-check-again = Check again
guard-rescue-use = Use what could be read

## R05, R06 undo

guard-undo-preview-title = What undo will do
guard-undo-preview-body = PCTwin looked at each change just now. Anything you changed after the move is kept. Nothing happens until you choose Undo.
guard-undo-preview-which = This undoes the move that finished on { DATETIME($when, year: "numeric", month: "long", day: "numeric", hour: "numeric", minute: "2-digit") }. Your other moves are not changed.
guard-undo-preview-which-earlier = This undoes the move made with an earlier version of PCTwin. Your other moves are not changed.
guard-undo-preview-admin = To remove apps, this laptop will ask an administrator to approve.
guard-undo-plan-put-back = Put back as it was
guard-undo-plan-remove-app = PCTwin removes the app. PCTwin does not touch files you made.
guard-undo-plan-keep-changed = Kept: you changed it
guard-undo-plan-elsewhere = Needs you: done elsewhere
guard-undo-plan-leave-alone = Left alone: it was already here
guard-undo-keep = Keep everything as it is
guard-undo-run = Undo
guard-undo-running-title = Undoing the changes
guard-undo-done-title = What undo did
guard-undo-result-restored = Restored
guard-undo-result-removed = Removed
guard-undo-result-as-before = Already as before
guard-undo-result-kept-changed = Kept because you changed it
guard-undo-result-kept-other = Kept, see More details
guard-undo-result-kept-at = Kept as { $name }
guard-undo-result-done-elsewhere = Done elsewhere: only its service or maker can undo it
guard-undo-result-not-done-yet = Not done yet. Undo tries again next time.

## The full move: apps, settings and app data (28 October 2026)

fullmove-plan-title = Apps, settings and app data
fullmove-plan-body = PCTwin looked at the apps, settings and app data on your old laptop. This is what it will do on this laptop after your files. Nothing changes until you start the move.
fullmove-group-install = Apps PCTwin will install: { $count }
fullmove-group-settings = App settings PCTwin will copy: { $count }
fullmove-group-needs = Things you will do yourself: { $count }
fullmove-group-here = Apps already on this laptop: { $count }
fullmove-settings-count = Settings: { $count }.
fullmove-settings-merge = They are added. Nothing on this laptop is replaced.
fullmove-settings-replace = Some settings on this laptop are replaced. Undo can put them back.
fullmove-app-install = PCTwin installs it from its maker and checks it.
fullmove-app-here = It is already on this laptop, so PCTwin leaves it as it is.
fullmove-step-copy = Copied, with undo.
fullmove-step-export-first = On your old laptop, use the app’s own export. Then import it in the app on this laptop.
fullmove-step-sign-in = Sign in to it again on this laptop.
fullmove-step-relink = Set it up again on this laptop, or link it again from the app.
fullmove-step-not-yet = PCTwin does not move this app’s data yet. It is still safe on your old laptop.
fullmove-step-look-at-it = PCTwin could not tell what this is. Look at it on your old laptop.
fullmove-step-convert = PCTwin brings this mail over and writes the mail kept only on your old Mac as mailbox files on this laptop. You then import them in your mail app.
fullmove-group-update =
    { $count ->
        [one] An older version is on this laptop
       *[other] Older versions are on this laptop
    }
fullmove-update-it = Update { $name }
fullmove-update-hint = PCTwin installs the newer version. Your settings for it stay.
fullmove-special-not-yet = App data such as email, browsers and games
fullmove-left-by-person = Apps, settings and app data: left on your old laptop, as you chose
fullmove-special-finish = PCTwin did not move this. It is still safe on your old laptop.
fullmove-more = And { $count } more.
fullmove-not-moved-title = Settings PCTwin leaves on the old laptop
fullmove-not-moved-private = Private and security settings, and sign-ins, which are never copied: { $count }
fullmove-not-moved-could-run = Settings that would start or open something, kept back for safety: { $count }
fullmove-not-moved-old-laptop-only = Settings that only fit the old laptop: { $count }
fullmove-not-moved-system-own = The system’s own settings that PCTwin does not move yet: { $count }
fullmove-not-moved-not-read = Settings found but not read: { $count }
fullmove-special-not-moved = App files left behind, such as caches: { $count }
fullmove-prompt = This laptop will ask once for an administrator’s permission.
fullmove-prompt-restore = This laptop will ask once for an administrator’s permission, and makes a restore point before the first change.
fullmove-rebuilding-title = Setting up your apps and settings
fullmove-rebuilding-body = Your files are in place. PCTwin is now setting up the rest. Keep this laptop on.
fullmove-rebuilding-permission = Asking this laptop for permission
fullmove-rebuilding-apps = Apps: { $done } of { $total }
fullmove-rebuilding-settings = Settings: { $done } of { $total }
fullmove-outcome-title = Apps, settings and app data
fullmove-outcome-undo = Undo app settings
fullmove-outcome-stopped = PCTwin stopped changing settings part way, because it could not write down its next change. Nothing after that point was changed.
fullmove-restore-made = A restore point was made before the changes.
fullmove-restore-earlier-exists = No new restore point was made, because this laptop already had one from today.
fullmove-restore-not-made = No restore point was made. PCTwin’s own undo still works.
fullmove-restore-not-windows = This laptop has no restore points. PCTwin’s own undo still works.
guard-undo-remove-yourself = Remove it yourself if you want
guard-undo-plan-remove-yourself = You can remove it in your laptop’s settings
guard-undo-result-remove-yourself = Left for you to remove in your laptop’s settings
guard-undo-plan-keep-other-version = Kept: it changed since the move
guard-undo-plan-already-gone = Already removed
guard-undo-result-kept-other-version = Kept because it changed since the move
guard-undo-result-removed-restart = Removed. Restart your laptop to finish.
guard-undo-result-not-removed = Not removed. You can remove it in your laptop’s settings.
guard-undo-close = Done
move-wait-appdata-title = Looking at your app data
move-wait-appdata-body = Your files are found. PCTwin is still looking at the data your apps keep, such as email and browsers. This takes at most ten minutes. Nothing on this laptop is changed.
fullmove-special-not-checked = App data PCTwin could not fully check
fullmove-group-appdata = App data PCTwin will copy: { $count }
fullmove-group-secrets = Secret files you can choose to bring: { $count }
fullmove-secrets-body = These hold passwords or keys. PCTwin brings one only if you tick it yourself. Left unticked, it stays on your old laptop.
fullmove-secret-bring = Bring { $name }
fullmove-secret-hint = Only you can open it on this laptop. Undo can remove it again.
fullmove-appdata-title = Copying your app data
fullmove-appdata-progress = Copying app data: { $done } of { $total } apps
fullmove-appdata-waiting = Waiting for the old laptop: close { $apps } there.
fullmove-close-title = Close these apps to bring their data
fullmove-close-body = These apps are still open on this laptop, so PCTwin cannot copy their data safely. Close them, then choose Try again. If you go on without them, their data stays here.
fullmove-close-again = Try again
fullmove-close-without = Continue without them
fullmove-extras-refused = Settings and app data from the old laptop could not be read here, so none were moved. They are still on your old laptop.
fullmove-app-from = PCTwin installs the winget package { $id } from its maker, after checking it is this app and the same maker.

## The full move: what each app and its data still needs (28 October 2026)

fullmove-here-older = An older version ({ $version }) is on this laptop. PCTwin leaves it as it is.
fullmove-step-sites = Saved sign-ins to websites stay tied to the old laptop. Sign in to the websites again on this laptop.
fullmove-app-rosetta = Made for older Intel Macs. On this Mac it runs through Rosetta, which Apple is ending, so it may stop working after a later macOS update. Look for a newer version from its maker.
fullmove-links = Shortcuts to other places among your files, which PCTwin does not follow or move: { $count }
fullmove-note-sign-in = Open it and sign in to your account.
fullmove-note-licence-key = Open it and enter your licence key, or sign in to the account that holds your licence.
fullmove-note-registration = Your old laptop listed it as registered to { $who }.
fullmove-note-product-id = Its product ID on your old laptop was { $id }.
fullmove-note-extras = Other programs came with it: { $apps }. Remove them in your laptop’s settings if you do not want them.
fullmove-note-rosetta = Made for Intel Macs, so it needs Rosetta. If this Mac offers to install Rosetta when you open the app, choose Install.

fullmove-note-came-from-maker = Your old laptop’s copy of its installer came from the maker’s own website.
fullmove-note-came-from-elsewhere = Your old laptop’s copy of its installer did not come from the maker’s own website. PCTwin gets apps only from the maker.
fullmove-data-lost = The connection was lost before this data arrived. It is still safe on your old laptop. Start a new move to bring it.
fullmove-data-did-not-arrive = This data did not arrive. It is still safe on your old laptop. Start a new move to bring it.
fullmove-data-app-open = The app was open on your old laptop, so its data was not copied. Close the app there, then start a new move to bring it.
fullmove-data-changing = This data kept changing on your old laptop while PCTwin read it. Close the app there, then start a new move to bring it.
fullmove-data-unreadable = PCTwin could not read this data on your old laptop. It is still there.
fullmove-data-not-read = The move stopped before this data was read. It is still safe on your old laptop.
fullmove-data-not-whole = Not all of this data arrived, so PCTwin put none of it in place. It is still safe on your old laptop.
fullmove-data-not-landed = This data arrived, but PCTwin could not put it in place on this laptop. It is still safe on your old laptop.
fullmove-data-beside = The app already had its own folder on this laptop, so PCTwin put the old laptop’s copy beside it, in { $folder }. The app still uses its own folder. To use the old copy instead, close the app and swap the two folders.
fullmove-data-placed = The data is in { $folder }, but the app needs a step before it can use it. Open the app and check it.
fullmove-data-mail-ready = Messages ready: { $messages }, in { $folder }. Open them in your mail app. In Apple Mail: File, Import Mailboxes, then choose that folder. In Thunderbird: the ImportExportTools NG add-on, Import mbox file.
fullmove-data-mail-parts-missing = Messages in { $folder }: { $messages }. Without the attachments the old Mac kept apart: { $missing }. Could not be written: { $failed }. All of it is still on your old Mac.
fullmove-data-mail-sign-in = Mail accounts whose mail stays on their server: { $accounts }. Sign in to them in your mail app on this laptop.
fullmove-data-mail-no-access = PCTwin could not look inside your mail on the old Mac, so mail kept only there did not move. Turn on Full Disk Access for PCTwin on the old Mac (System Settings, Privacy & Security), then start a new move.
fullmove-outcome-here-title = Apps already on this laptop: { $count }
fullmove-outcome-here = Already on this laptop, left as it is.
fullmove-outcome-here-older = An older version ({ $version }) is on this laptop. PCTwin left it as it is. Update it from its maker if you want the newer one.
fullmove-outcome-here-badge = Left as it is
fullmove-outcome-fresh = Apps PCTwin installed start fresh. Open each one and sign in if it asks.
progress-may-sleep = This laptop may go to sleep during the move. Keep it plugged in and open.

## The old laptop chooses what moves (decided 28 October 2026)
choose-title = What should move?
choose-body = Choose here, on the laptop with your things. Your new laptop then shows what is coming and asks you to accept it.
choose-mode = What to move
choose-everything = Everything
choose-except = Everything except
choose-only = Only what I choose
choose-except-hint = Untick what to leave behind.
choose-open = Open { $name }
choose-close = Close { $name }
choose-total =
    { $files ->
        [one] 1 file
       *[other] { $files } files
    } · Chosen: { $size }
choose-only-hint = Tick what to bring.
choose-apps = Apps, settings and app data
choose-apps-count =
    { $count ->
        [one] 1 app, with its settings
       *[other] { $count } apps, with their settings
    }
choose-group-size =
    { $files ->
        [one] 1 file
       *[other] { $files } files
    } · { $size }
choose-left-title = Left behind anyway
choose-left-badge = Left behind
choose-left-caches = Caches and temporary files
choose-left-chosen-by-user = Things you chose to leave
choose-left-work-managed = Your work or school's files
choose-left-device-bound = Things tied to this laptop, such as passkeys
choose-left-unreadable = Files PCTwin could not read
choose-left-cloud-only = Files kept only online
choose-left-system = Parts of Windows itself
choose-left-too-many-files = Folders too big for one move
choose-left-rebuildable = Things your new laptop can make again, such as downloaded packages
choose-nothing = Choose at least one thing to move.
choose-continue = Continue

## The new laptop accepts what the old laptop chose (decided 28 October 2026)
accept-title = Accept the move from { $from }?
accept-body = This was chosen on your old laptop. Accept it, and PCTwin moves it here.
accept-files =
    { $count ->
        [one] 1 file
       *[other] { $count } files
    }
accept-apps =
    { $count ->
        [one] 1 app to install
       *[other] { $count } apps to install
    }
accept-settings = Settings and app data
accept-fits = It fits on this laptop.
accept-needs = Something needs you first. See the details.
accept-not-now = Not now
accept-details = See the details
accept-accept = Accept

# 2.5 special categories
special-category-email = Email
special-category-browser = Browsers
special-category-game = Games
special-category-ai-tool = AI tools and code editors
special-category-secret = Keys and passwords
special-category-other = Other app data
special-note-search-rebuilds = Your mail app builds its search again the first time it opens. Searching may be slow for a while.
special-note-server-resync = Your mail comes back from the mail server when you sign in. PCTwin keeps the old copy only in case something never reached the server.
special-note-own-sync = The easiest way: turn on this browser's own sync on your new laptop. It brings your bookmarks, history, open tabs and add-ons.
special-note-passwords-guided = Saved passwords move only with the browser's own export and import, or its sync. The exported file is not protected, so PCTwin keeps it private and removes it after the import.
special-note-passkeys-guided = Passkeys saved in an account come back when you sign in to it. Passkeys kept only on this laptop must be made again on each website. Do this before you wipe the old laptop.
special-note-cloud-saves-may-differ = The game's launcher may also keep your saves online. If it asks which saves to keep, check the dates first.
special-note-models-checked-not-run = AI models are checked file by file when they arrive. PCTwin never runs them.
special-note-tool-settings-review = These settings can start programs or reach the internet. Check them before you use them on the new laptop.
special-note-secret-tick-each = This moves only if you tick it. It never appears in a report.
browser-file-bookmarks = Bookmarks.html
browser-file-lists = Open pages and add-ons.html
browser-file-bookmarks-title = Bookmarks
browser-file-add-ons = Add-ons
browser-file-open-pages = Open pages
mail-file-folder = Mail from { $laptop }
mail-file-on-this-mac = On My Mac
mail-file-account = Mail account { $n }
special-note-open-archive-in-mail-app = Open this mail archive in your mail app on the new laptop. In Outlook: File, Open, Outlook Data File.
special-note-local-only-folders-export = Folders marked “This computer only” are not on the mail server. Before you wipe the old laptop, export them from your mail app to an archive file, then move that file.
special-note-mail-import = Mail that stays on a mail server comes back when you sign in. To open the mailbox files: in Apple Mail, choose File, Import Mailboxes. In Thunderbird, use the ImportExportTools NG add-on.
special-note-mail-no-access = PCTwin could not look inside your mail on the old Mac. Turn on Full Disk Access for PCTwin there (System Settings, Privacy & Security) to bring mail kept only on that Mac.
special-note-bookmarks-file = Your bookmarks are also saved as a file that any browser can import. If the new laptop has a different browser, choose its option to import bookmarks from an HTML file, then choose this file. You find it in the folder "PCTwin brought over".
special-note-history-own-import = Different browsers do not share one format for history. To move history to a different browser, use that browser's own import while both browsers are on one laptop, or turn on the browser's sync.
special-note-pages-and-add-ons-listed = A list of your add-ons and the pages you had open is saved next to the bookmarks file. It is only for you. PCTwin does not open any of these pages.
special-note-open-pages-by-sync = PCTwin cannot list the pages you had open in this browser. To open them again, turn on the browser's sync, or its setting to open the pages from last time.
forgotten-title = Things people often forget
forgotten-kind-fonts = Your own fonts
forgotten-kind-mail-signatures = Mail signatures
forgotten-kind-dictionaries = Your own dictionary words
forgotten-kind-office-templates = Office templates
forgotten-kind-mail-archives = Mail kept on this laptop
forgotten-kind-game-saves = Game saves
forgotten-kind-game-library = Games installed by a game launcher
forgotten-moving = This moves with your other things.
forgotten-not-moving = This is not in the move. You can copy it yourself.
forgotten-step-install = After the move, open each font on the new laptop and choose Install.
forgotten-step-open-in-mail-app = Open this mail archive in your mail app on the new laptop. In Outlook: File, Open, Outlook Data File.
forgotten-step-export-first = PCTwin writes the mail kept only on the old Mac as mailbox files on the new laptop. You then import them in your mail app.
forgotten-step-locate-verify = Install the game launcher on the new laptop and sign in. It can install your games again. If you copy this folder over, use the launcher's option to find installed games, and let it check the files.
forgotten-not-whole = PCTwin stopped counting here, so there may be more.
forgotten-printers = Printers to add again
forgotten-printer-step = Add this printer again on your new laptop.
forgotten-printer-network = Shared from another computer
forgotten-not-checked = PCTwin could not look here.
forgotten-files =
    { $count ->
        [one] 1 file
       *[other] { $count } files
    }
forgotten-online-only =
    { $count ->
        [one] 1 file kept only online, not downloaded
       *[other] { $count } files kept only online, not downloaded
    }

# 2.7 finishing a move
doctor-not-connected = This laptop is not connected to any network.
doctor-not-connected-fix = Connect to Wi-Fi or plug in a network cable, then try again.
doctor-mac-local-network-denied = This Mac may not be letting PCTwin talk to devices on your network.
doctor-mac-local-network-denied-fix = Open System Settings, then Privacy & Security, then Local Network, and turn on PCTwin.
doctor-discovery-blocked = Something on this laptop, such as a firewall or security app, may be stopping the laptops from finding each other.
doctor-discovery-blocked-fix = Allow PCTwin in your firewall or security app, or pause it while you move.
doctor-windows-public-network = Windows treats the network { $name } as public, which blocks other devices.
doctor-windows-public-network-fix = Open Settings, then Network & internet, choose { $name }, and set it to a private network.
doctor-vpn-on = A VPN is on ({ $name }). It can hide your home network.
doctor-vpn-on-fix = Turn off the VPN while you move, then try again.
doctor-old-laptop-not-pairing = The old laptop may not be showing its code.
doctor-old-laptop-not-pairing-fix = On the old laptop, open PCTwin and choose that it is your old laptop.
doctor-different-networks = The laptops may be on different networks.
doctor-different-networks-fix = Put both laptops on the same Wi-Fi.
doctor-guest-network = This network may stop devices from seeing each other, as guest, hotel and office Wi-Fi often do.
doctor-guest-network-fix = Use your home Wi-Fi, or your phone’s hotspot.
doctor-probe-interfaces = PCTwin could not list this laptop’s network connections.
doctor-probe-hears-itself = PCTwin could not test whether this laptop can be found on the network.
doctor-probe-local-network = PCTwin could not check the Mac’s local network setting.
doctor-probe-network-profiles = PCTwin could not check how Windows treats this network.
doctor-hotspot = Still not working? Turn on your phone’s hotspot and join both laptops to it. The move goes straight between the laptops, not through your phone’s data.
windows-make-other-administrator = Make sure the person keeping the laptop has an administrator account on it.
windows-sign-in-other-administrator = Sign out, then sign in with that other administrator account. An account cannot remove itself.
windows-open-other-users = Open Settings, then Accounts, then Other users.
windows-remove-account = Choose your account, then Remove.
windows-delete-account-and-data = Choose Delete account and data. Everyone else’s account stays as it is.
windows-sign-out-of-everything = First sign out of your apps and accounts on the old laptop.
windows-open-recovery = Open Settings, then System, then Recovery.
windows-reset-this-pc = Choose Reset PC.
windows-remove-everything = Choose Remove everything.
windows-clean-the-drive = Change the settings so the drive is cleaned, not just emptied. This takes a few hours.
windows-wait-until-setup-screen = Wait until the laptop shows its setup screen. Then it is ready to hand on.
macos-make-other-administrator = Make sure the person keeping the Mac has an administrator account on it.
macos-sign-in-other-administrator = Log out, then log in with that other administrator account. An account cannot remove itself.
macos-open-users-and-groups = Open System Settings, then Users & Groups.
macos-delete-user = Choose the information button next to your account, then Delete User.
macos-delete-home-folder = Choose to delete the home folder.
macos-sign-out-of-apple-account = First sign out of your Apple Account on the old Mac, so it is free for its next owner.
macos-open-transfer-or-reset = Open System Settings, then General, then Transfer or Reset.
macos-erase-all-content-and-settings = Choose Erase All Content and Settings, and follow the steps.
macos-older-mac-use-recovery = If your Mac does not offer this (older Macs), start it in macOS Recovery, erase the drive with Disk Utility, then reinstall macOS.
macos-wait-until-setup-screen = Wait until the Mac shows its setup screen. Then it is ready to hand on.
linux-sign-in-other-administrator = Sign in with another administrator account.
linux-open-users = Open Settings, then Users.
linux-remove-user-and-files = Choose your account, then Remove User, and choose to delete its files.
linux-no-built-in-reset = Linux has no built-in reset that erases the drive.
linux-drive-maker-secure-erase = Use the drive maker’s secure erase tool, or the laptop’s own erase in its startup settings, if it has one.
linux-reinstall-from-usb = Then install a fresh system from a USB stick.
any-sign-in-other-administrator = Sign in with another administrator account on the old laptop.
any-use-system-account-settings = Remove your account with the system’s own account settings.
any-use-system-reset = Use the laptop’s own reset that erases the drive. The laptop maker’s help says how.
any-drive-maker-secure-erase = If there is none, use the drive maker’s secure erase tool.

# 2.9 deliverables
guard-undo-removes-if-package = PCTwin removes it if it came as an installer package; otherwise remove it yourself
guard-undo-keeps-newer = Undo keeps the newer version: the older one cannot come back
compat-outcome-works = Tested: works
compat-outcome-works-with-steps = Tested: works, with a few steps from you
compat-outcome-partly = Tested: some parts work
compat-outcome-not-supported = Tested: PCTwin cannot set this up here
compat-outcome-not-tested = Not tested yet
compat-aspect-install = Installing
compat-aspect-data-import = Bringing its data
compat-aspect-settings = Its settings
compat-aspect-launch = Opening it
compat-aspect-undo = Undo
compat-result-works = Works
compat-result-guided = Works with your help
compat-result-not-supported = Does not work
compat-result-not-applicable = Not needed
compat-result-not-tested = Not tested
compat-step-sign-in = Sign in
compat-step-licence = Enter the licence key
compat-step-import-in-app = Import your data in the app
compat-step-relink = Point it at your files again
compat-step-restart = Restart the laptop
compat-how-lab = Tested by PCTwin on clean laptops
compat-how-real-laptops = Tested on real laptops
work-what-linked-files = Linked files
work-what-font = Font
work-what-archive = Archive opens
work-what-database = Database opens
work-what-task = Something you check yourself
work-label-machine-checked = Checked by PCTwin
work-label-person-confirmed = You checked it
work-label-still-needs-attention = Still needs checking
work-state-not-run-yet = Not checked yet
work-state-all-there = Everything is there
work-state-missing = Something is missing
work-state-damaged = Not whole
work-state-not-there = Not there
work-state-waiting-for-you = Waiting for you
work-state-you-said-no = You said it did not work
work-state-outside-project = Links outside its folder: not looked at
work-state-could-not-read = Could not be read
work-state-not-supported = PCTwin cannot check this kind of file yet
work-state-partly-checked = Not everything was looked at
work-problem-not-chosen = No file was chosen.
work-problem-too-many = Too many checks at once. Remove some first.
work-problem-no-such-check = That check is no longer there.
work-problem-stale = The list changed. Look at it again.
work-problem-bad-name = Type a name, up to 200 characters.
work-problem-wrong-kind = That answer does not fit this check.
work-problem-failed = Something went wrong. Try again.
repair-waits-old-laptop = Needs the old laptop connected
repair-waits-internet = Needs the internet
repair-waits-administrator = Needs an administrator’s approval
repair-waits-installer-tool = Needs the system's app installer
repair-waits-space = Needs more room on this laptop
repair-waits-close-app = Needs the app closed
repair-problem-nothing-to-repair = Nothing picked needs fixing.
repair-problem-stale = The list changed. Look at it again.
repair-problem-busy = PCTwin is busy with something else. Try again when it finishes.
repair-problem-failed = Something went wrong. Nothing else was changed.
changed-why-app-updated = The app was updated after you approved
changed-why-app-removed = The app was removed after you approved
changed-why-destination-changed = The folder it goes to changed
changed-why-project-edited = It was changed on the old laptop after you approved
changed-step-install = Installing
changed-step-settings = Settings
changed-step-app-data = App data
changed-step-files = Files
safe-stop-busy-moving = Moving files
safe-stop-busy-rebuilding = Setting up apps and settings
safe-stop-busy-app-data = App data
safe-stop-busy-undoing = Undoing a move
kit-part-move-record = Your move record
kit-part-results = The result of every item
kit-part-before-wipe-list = What is still only on the old laptop
kit-part-file-data = Your files
kit-part-setup-steps = The steps to set apps up again
kit-part-recovery-tool = A tool to restore things

# 2.8 screens
report-group-apps = Apps
report-group-settings = Settings
report-group-special = App data and other things
report-group-people = Other people
report-count-apps = { $count ->
        [one] 1 app
       *[other] { $count } apps
    }
report-count-settings = { $count ->
        [one] 1 group of settings
       *[other] { $count } groups of settings
    }
report-count-special = { $count ->
        [one] 1 thing
       *[other] { $count } things
    }
report-count-people = { $count ->
        [one] 1 person
       *[other] { $count } people
    }
report-beyond-done = All done and checked
report-beyond-some = { $done } of { $total } done and checked
report-beyond-left = { $count ->
        [one] 1 not moved: you left it out
       *[other] { $count } not moved: you left them out
    }
report-beyond-more = { $count ->
        [one] 1 more is not shown here.
       *[other] { $count } more are not shown here.
    }
report-here = { $count ->
        [one] 1 app was already on this laptop. PCTwin left it as it is.
       *[other] { $count } apps were already on this laptop. PCTwin left them as they are.
    }
progress-ready-title = Ready to use now
progress-ready-body = { $count ->
        [one] { $groups } is copied and checked. You can open it on this laptop while the rest comes.
       *[other] { $groups } are copied and checked. You can open them on this laptop while the rest comes.
    }
finish-strip = Finish your move
finish-title = Finish your move
finish-loading = Looking at your move.
finish-retry = Try again
finish-back-to-list = Back to the list
finish-open = Open
finish-open-name = Open { $title }
finish-more-not-shown = More not shown: { $count }
finish-problem-no-move = PCTwin has no move to finish on this laptop. Nothing was changed.
finish-problem-changed = What is shown is out of date. Look at it again before you answer. Nothing was changed.
finish-problem-not-allowed = That cannot be done. Nothing was changed.
finish-problem-busy = A move or another step is going on. Wait until it ends, then try again. Nothing was changed.
finish-problem-failed = Something went wrong on this laptop. Nothing was changed. Try again.
finish-headline-ready-for-selected-work = Everything you picked is on this laptop
finish-headline-ready-for-selected-work-body = PCTwin found each item you picked on this laptop and checked it as far as it can. This does not promise that every app works as it did before.
finish-headline-finished-with-tasks = The move is finished, and some tasks are left
finish-headline-finished-with-tasks-body = Some things need you, or did not copy. They are listed on this page.
finish-headline-data-preserved = Everything is here, but some of it needs a key to open
finish-headline-data-preserved-body = Everything is on this laptop. A key or recovery code is needed to open some of it, and PCTwin could not check that here.
finish-headline-handover-unresolved = The old laptop is not handed over yet
finish-headline-handover-unresolved-body = This laptop is done. The list for leaving the old laptop still has open things.
finish-headline-nothing-moved = Nothing was picked to move
finish-headline-nothing-moved-body = No files, apps or settings were picked, so nothing was copied.
finish-tally-title = What moved
finish-tally-selected = Picked: { $count }
finish-tally-done = Copied and checked: { $count }
finish-tally-needs-action = Need you: { $count }
finish-tally-failed = Did not copy: { $count }
finish-tally-waiting = Still waiting: { $count }
finish-tally-not-reachable = Old laptop not reachable: { $count }
finish-tally-no-result-yet = No result yet: { $count }
finish-tally-skipped = Skipped by you: { $count }
finish-tally-left-out = Left out: { $count }
finish-tally-accepted-loss = Left behind on purpose: { $count }
finish-others = Other people’s items, counted only and never shown: { $count }
finish-after-title = Checked on this laptop now
finish-after-body = PCTwin counted these on the old laptop, then counted again on this laptop.
finish-after-count = { $onNew } of { $atScan } are on this laptop
finish-after-all = All there
finish-after-short = Some missing
finish-group-photos = Photos
finish-group-videos = Videos
finish-group-music = Music
finish-group-documents = Documents
finish-group-other-files = Other files
finish-group-apps = Apps
finish-group-settings = Settings
finish-note-parental-controls = Parental controls and family safety settings do not move. Set them up again on this laptop.
finish-note-not-everywhere = Some places on the old laptop were not searched.
finish-note-results-refused = Some results were not accepted, because this laptop did not show them as done.
finish-note-undo-open = Undo is still open for this move. It closes for good if you erase the old laptop.
finish-open-title = Not finished
finish-open-body = These are your items that still need attention.
finish-standing-copied = Copied
finish-standing-reinstalled = Reinstalled
finish-standing-converted = Converted
finish-standing-needs-action = Needs you
finish-standing-failed = Did not copy
finish-standing-waiting = Waiting
finish-standing-not-reachable = Old laptop not reachable
finish-standing-no-result-yet = No result yet
finish-standing-skipped-by-user = Skipped by you
finish-standing-left-out = Left out
finish-access-key-not-checked = A key or code to open it was not checked here.
finish-access-not-on-new-laptop = It is not on this laptop.
finish-search-title = Where PCTwin looked on the old laptop
finish-search-searched = Looked in: { $places }
finish-search-not-searched = Did not look in: { $places }
finish-search-all = PCTwin looked in every place it knows about. It cannot say there is nothing anywhere else.
finish-old-checked = The old laptop was last checked: { $when }
finish-old-never = The old laptop has not been checked since the move.
finish-steps-title = Next steps
finish-step-setup = Finish setting up
finish-step-setup-detail = Apps that need you to sign in or finish a step.
finish-step-goodbye = Before you leave the old laptop
finish-step-goodbye-detail = What is still only on the old laptop.
finish-step-catchup = Copy recent changes
finish-step-catchup-detail = Copy what changed on the old laptop since the move.
finish-step-handover = Hand the old laptop on
finish-step-handover-detail = Remove your account or erase the old laptop, with the system’s own tools.
finish-step-undo = Undo this move
finish-step-undo-detail = Remove apps and put settings back.
finish-step-cleanup = Clean up
finish-step-cleanup-detail = Remove PCTwin’s temporary files from this laptop.
finish-step-report = Share the report
finish-step-report-detail = Save a plain report. You choose what it includes.
finish-step-doctor = Check the connection
finish-step-doctor-detail = Find out why the two laptops cannot connect.
finish-setup-title = Finish setting up
finish-setup-body = These apps need a step from you. A tick records what you say. It does not count as a PCTwin check.
finish-setup-empty = Nothing is waiting for you.
finish-setup-sign-in = Sign in to { $name }
finish-setup-licence-key = Enter the licence key for { $name }
finish-setup-set-up-again = Set up { $name } again
finish-setup-download-from-maker = Download { $name } from its maker’s website
finish-setup-permission = Give { $name } permission when it asks
finish-setup-restart = Restart this laptop to finish { $name }
finish-setup-finish-by-hand = Finish { $name } by hand
finish-setup-parts-missing = Some parts of { $name } did not arrive. Send them again
finish-setup-export-first = Export from { $name } on the old laptop first
finish-setup-relink = Link { $name } to your account again
finish-setup-look-at-it = Look at { $name } and check it is right
finish-setup-screens = Check the text size and screen layout in this laptop’s display settings
finish-setup-open-once = Open { $name } once and check it starts
finish-setup-mark-open = Not done
finish-setup-mark-machine-checked = PCTwin checked
finish-setup-mark-you-did-it = You said it is done
finish-setup-tick-hint = Tick when you have done it.
finish-setup-save-row = Save your app list and setup
finish-setup-save-row-detail = Keep a list of your apps and settings in My PCTwin history.
finish-save-title = Save your app list and setup
finish-save-body = PCTwin keeps a list of your apps and settings in My PCTwin history, on this laptop only. It is a list, not a copy of your files.
finish-save-button = Save the list
finish-save-saved = Saved in My PCTwin history. Apps: { $apps }. Settings: { $settings }.
finish-save-earlier-title = Saved before
finish-save-earlier = Saved: { $when }
finish-goodbye-title = Before you leave the old laptop
finish-goodbye-body = This is what is still only on the old laptop. Answer each one. Nothing is erased here. Erasing is a separate step.
finish-goodbye-saved = This list is saved on this laptop.
finish-goodbye-only-on-old-laptop = Only on the old laptop: { $count }
finish-goodbye-key-not-checked = Items that need a key or code that was not checked: { $count }
finish-goodbye-tied-to-old-laptop = Tied to the old laptop: { $count }
finish-goodbye-do-first-on-old-laptop = To do on the old laptop first: { $count }
finish-goodbye-steps-not-finished = Steps not finished: { $count }
finish-goodbye-not-searched = Places not searched: { $count }
finish-goodbye-old-laptop-not-reachable = The old laptop could not be reached
finish-goodbye-special-sign-in = { $names }: sign in again
finish-goodbye-special-relink = { $names }: link it to your account again
finish-goodbye-special-export-first = { $names }: export what you need from the old laptop first
finish-goodbye-special-look-at-it = { $names }: look at it and check it is right
finish-goodbye-places = Places: { $places }
finish-goodbye-did-it = I did it
finish-goodbye-accept = I accept it is lost
finish-goodbye-accept-warning = If you accept this, it stays only on the old laptop. When the old laptop is erased, it is gone.
finish-goodbye-cannot-answer = Only a new check of the old laptop can settle this. Connect it and check again.
finish-goodbye-res-open = Not answered yet
finish-goodbye-res-person-did-it = You said you did it
finish-goodbye-res-accepted-loss = You accepted it is lost
finish-goodbye-answer-label = Your answer for: { $title }
finish-goodbye-keep-title = Keep the old laptop untouched until
finish-goodbye-keep-body = Suggested date: { $date }. Until then, do not erase it, in case you need something from it.
finish-goodbye-keep-label = Keep until
finish-goodbye-keep-use = Use the suggested date
finish-goodbye-keep-save = Save this date
finish-goodbye-keep-set = Kept until: { $date }
finish-goodbye-keep-none = No date chosen yet
finish-goodbye-blocked = The old laptop cannot be handed on yet. Still open: { $count }.
finish-goodbye-blocked-unchecked = The old laptop has not been checked since the move. Check it again first.
finish-goodbye-clear = Everything is answered and the old laptop was checked. You can ask for a handover. That is a separate step.
finish-goodbye-handover = Hand the old laptop on
finish-goodbye-check-again = Check the old laptop again
finish-catchup-title = Copy recent changes
finish-catchup-body = See what changed on the old laptop since the move. You review each change. PCTwin sends only the ones you choose.
finish-catchup-purpose-check = Only look at the old laptop
finish-catchup-purpose-check-detail = Nothing is copied. Your before-you-leave list is updated.
finish-catchup-purpose-catch-up = Look and copy what changed
finish-catchup-purpose-catch-up-detail = You see the changes first and choose what is sent.
finish-catchup-armed = Ready. Connect the old laptop the same way as for the move. This page shows the changes when the old laptop answers.
finish-catchup-connect = Connect the old laptop
finish-catchup-waiting = Waiting for the old laptop.
finish-catchup-unreadable = PCTwin could not read the old laptop’s answer. Nothing was changed. Try again.
finish-catchup-changes-title = What changed
finish-catchup-check-only = Nothing is sent. This list only updates what PCTwin knows about the old laptop.
finish-catchup-none = Nothing changed on the old laptop.
finish-catchup-others = Changes to other people’s items, counted only: { $count }
finish-catchup-choose-label = What should PCTwin do with { $title }?
finish-catchup-send-label = Send { $title }
finish-catchup-nothing-to-send = Nothing to send
finish-change-new = New on the old laptop
finish-change-updated = Changed on the old laptop
finish-change-both-changed = Changed on both laptops
finish-change-gone-on-new-laptop = Removed from this laptop
finish-change-gone-on-old-laptop = Removed from the old laptop
finish-change-cannot-tell = PCTwin cannot tell
finish-choice-none = Do not send
finish-choice-send = Send it again
finish-choice-keep-both = Keep both copies
finish-choice-keep-new-laptop-version = Keep this laptop’s version
finish-catchup-approve = Send the changes I chose: { $count }
finish-catchup-sent = Sent for copying. Only the changes you chose are copied.
finish-handover-title = Hand the old laptop on
finish-handover-body = Each choice is a separate decision. Nothing happens until you confirm on its own page.
finish-handover-remove-title = Remove my account from the old laptop
finish-handover-remove-detail = Only your account. Other people’s accounts stay.
finish-handover-erase-title = Erase the old laptop
finish-handover-erase-detail = Everything on the old laptop’s drive.
finish-handover-preview-remove = Remove my account
finish-handover-preview-erase = Erase the old laptop
finish-handover-removes = What will be removed
finish-handover-keeps = What stays
finish-handover-keeps-none = Nothing on it is kept.
finish-handover-route = How it is done
finish-handover-route-body = PCTwin does not do this itself. The system’s own tool does it, and PCTwin hands you over to it.
finish-handover-continue = Continue to confirm
finish-handover-not-offered = This is not offered now
finish-handover-refused-open = Things still open on your before-you-leave list: { $count }
finish-handover-refused-nothing-done = Nothing was done.
finish-handover-to-list = Go to the before-you-leave list
finish-route-system-windows = Windows
finish-route-system-macos = macOS
finish-route-system-linux = Linux
finish-route-unknown-step = Follow the next step the system shows you.
finish-hnote-ssd-not-secure-wipe = Deleting files on an SSD is not a secure wipe.
finish-hnote-does-not-cancel-subscriptions = Removing an account does not cancel subscriptions or sign you out of other places.
finish-hnote-others-stay = Everyone else’s accounts stay as they are.
finish-hnote-cannot-be-undone = This cannot be undone.
finish-hnote-closes-undo = Going ahead closes undo for this move for good.
finish-hnote-sign-in-as-another-administrator = An account cannot remove itself. Sign in to the old laptop with another administrator account first.
finish-hnote-family-group-first = If the account is in a Microsoft family group, remove it from the family online first.
finish-hnote-not-a-certificate = PCTwin records what you asked for and what you tell it happened. That is not a certificate that anything was erased.
finish-hnote-encrypt-first-helps = Turning on the drive’s encryption before the reset makes what is left much harder to read.
finish-refused-wrong-device = This step is not available on this laptop.
finish-refused-undo-not-closed = Undo is still open for this move. It must be closed first.
finish-refused-old-laptop-not-checked = The old laptop was last checked more than 15 minutes ago. Check it again first.
finish-refused-list-out-of-date = The before-you-leave list is out of date. Open it again.
finish-refused-list-not-clear = The before-you-leave list still has open things.
finish-refused-no-write-exclusion = PCTwin could not make sure nothing is changing on the old laptop right now.
finish-refused-no-such-account = Your account was not found on the old laptop.
finish-refused-other-people-use-it = Other people use this account, so PCTwin will not remove it.
finish-refused-not-qualified = This old laptop cannot be handed on this way.
finish-refused-no-fresh-confirmation = A new confirmation is needed.
finish-refused-wrong-confirmation = That confirmation does not match what is shown now. Look again.
finish-refused-changed-since-confirmed = Something changed after you confirmed. Look again and confirm again.
finish-refused-expired = The confirmation ran out. Look again and confirm again.
finish-refused-already-used = This confirmation was already used.
finish-refused-nothing-to-remove = There is nothing to remove.
finish-confirm-title-remove-my-account = Confirm: remove my account
finish-confirm-title-erase-old-laptop = Confirm: erase the old laptop
finish-confirm-target = This will change: { $target }
finish-confirm-consequences = What this means
finish-confirm-tick-remove-my-account = I understand that the account { $target } will be removed from the old laptop and that this cannot be undone.
finish-confirm-tick-erase-old-laptop = I understand that everything on { $target } will be erased and that this cannot be undone.
finish-confirm-go = Confirm and open the system’s tool
finish-confirm-cancel = Go back and do nothing
finish-confirm-needs-tick = The button works after you tick the box above.
finish-started-title = Follow the system’s own steps
finish-started-body = The steps below are the system’s, not PCTwin’s. PCTwin does not show its own version of them.
finish-started-opened = PCTwin opened the system’s page.
finish-started-not-opened = PCTwin could not open the system’s page. Open it yourself and follow the steps.
finish-started-undo-closed = Undo is now closed for this move.
finish-says-title = Tell PCTwin what happened
finish-says-label = What did you see?
finish-says-choose = Choose what you saw
finish-says-done = I finished the steps
finish-says-partly = I finished part of the steps
finish-says-cancelled-part-way = I cancelled part way
finish-says-not-started = I did not start
finish-says-cannot-tell = I cannot tell
finish-says-record = Record what I saw
finish-recorded-person-says-done = Recorded: you said it finished. This is your word. It is not a certificate.
finish-recorded-partial = Recorded: only part of it was done.
finish-recorded-interrupted = Recorded: it was interrupted.
finish-recorded-cancelled-after-some-effects = Recorded: it was cancelled after some changes were made.
finish-recorded-not-started = Recorded: it was not started.
finish-recorded-inconclusive = Recorded: you could not tell what happened.
finish-recorded-time = Recorded: { $when }
finish-undo-title = Review before undo
finish-undo-body = This is what undo does and what it leaves. Nothing happens until you choose Start undo.
finish-undo-row-removes = Apps PCTwin installed: undo removes them
finish-undo-row-yours = Apps that are now yours: undo leaves them. Remove them yourself if you want.
finish-undo-row-settings = Settings put back as they were: { $count }
finish-undo-row-appdata = App data put back for these apps
finish-undo-row-files = Files the move copied: { $count }
finish-undo-limit-files = Only with the old laptop connected, and only copies nobody changed since.
finish-undo-limit-apps = Only while each app is still the version PCTwin installed.
finish-undo-keeps = What undo keeps: apps that are now yours, and anything you changed after the move.
finish-undo-closed = Undo is closed for this move. It cannot be used.
finish-undo-nothing = There are no apps or settings to undo for this move.
finish-undo-start = Start undo
finish-cleanup-title = Clean up
finish-cleanup-body = Each question is asked separately. Nothing is removed unless you confirm.
finish-cleanup-temp = PCTwin’s temporary files
finish-cleanup-temp-detail = Files PCTwin made on this laptop while it worked.
finish-cleanup-model = PCTwin’s model and saved data
finish-cleanup-model-detail = Helper files PCTwin keeps on this laptop so it works faster.
finish-cleanup-look = Look
finish-cleanup-removes = Will be removed
finish-cleanup-keeps = Will be kept
finish-cleanup-keeps-body = These only look like PCTwin’s, or are in use.
finish-cleanup-nothing = There is nothing to remove.
finish-cleanup-tick = Remove the items listed under Will be removed
finish-cleanup-confirm = Remove them
finish-cleanup-done = Removed: { $removed }. Kept: { $kept }. Could not remove: { $failed }.
finish-doctor-title = Check the connection
finish-doctor-body = PCTwin looks at this laptop to find why the two laptops cannot connect.
finish-doctor-role-label = Which laptop is this?
finish-doctor-role-old = The old laptop
finish-doctor-role-new = The new laptop
finish-doctor-run = Run the check
finish-doctor-running = Checking this laptop.
finish-doctor-found = What PCTwin found
finish-doctor-none = PCTwin found no problem on this laptop. That does not prove the connection will work.
finish-doctor-sure = Sure
finish-doctor-likely = Likely
finish-doctor-fix = What to do
finish-doctor-not-checked = PCTwin could not check
finish-notchecked-unknown = One check could not be made on this laptop.
finish-report-title = Share the report
finish-report-body = Choose what the report includes. Nothing is included unless you choose it. Nothing is sent. You save the text you read, and share it yourself.
finish-report-choices = Include in the report
finish-report-file-names = File names
finish-report-app-names = App names
finish-report-account-id = Account ID
finish-report-preview = The report as it will be saved
finish-report-withheld = Kept out of the report: names { $names }, secrets { $secrets }, account IDs { $ids }.
finish-report-save = Save the report
finish-report-saved = Saved to: { $path }
finish-report-not-saved = No file was saved.
finish-report-feedback = To tell the PCTwin team what happened, use Help, then Feedback. No account is needed, and nothing is sent automatically.
finish-report-help = Open Help
finish-report-text-title = PCTwin move report
finish-report-text-unnamed-file = A file
finish-report-text-unnamed-app = An app
finish-report-text-account = Account ID: { $id }
finish-report-text-others = Other people’s items not finished, counted only: { $count }
finish-report-text-goodbye = Before-you-leave list: { $total } things, { $open } still open
finish-report-text-lines = Items not finished
rescue-eyebrow = Rescue a laptop
rescue-title = Rescue a laptop
rescue-lead = Bring files back from a laptop that will not start. Connect its drive to this laptop, then choose it here.
rescue-checking = Looking for connected drives.
rescue-failed = PCTwin could not list the drives connected to this laptop. Check the cable, then check again.
rescue-empty = No drive connected. Connect the old laptop’s drive with a USB adapter, then check again.
rescue-looks-like-laptop = Looks like a laptop’s drive
rescue-choose = Choose { $name }
rescue-check-again = Check again
rescue-locked-title = If the drive is locked
rescue-locked-windows = If the drive is locked with BitLocker, Windows asks for its recovery key in its own window when you open the drive.
rescue-wait-title = Reading { $drive }
rescue-wait-body = PCTwin is looking through the folders on { $drive }. Nothing on the drive is changed.
rescue-accept-title = Files found on { $drive }
rescue-accept-body = Accept, and PCTwin copies them to this laptop. Nothing on the drive is changed.
rescue-progress-title = Copying files from { $drive }
rescue-safe = The drive is only read. Nothing on it is changed.
rescue-outcome-title = Files copied from { $drive }
plan-net-title = What needs the internet
plan-net-none = Nothing in this move needs the internet.
plan-net-downloads =
    { $count ->
        [one] PCTwin downloads { $count } app from the internet.
       *[other] PCTwin downloads { $count } apps from the internet.
    }
plan-net-sizes = The size of each download is known only once it has been downloaded.
plan-net-signins =
    { $count ->
        [one] You sign in again online to { $count } thing.
       *[other] You sign in again online to { $count } things.
    }
plan-room-title = Room needed beyond your files
plan-room-body = At the busiest moment PCTwin needs about { $installs } for apps and about { $appData } for app data, on top of your files.
compat-strip = What was tested
compat-title = { $name }: what was tested
compat-open = Details
compat-open-name = See what was tested for { $name }
compat-list-title =
    { $count ->
        [one] How well { $count } app was tested
       *[other] How well { $count } apps were tested
    }
compat-aspects-title = Each part
compat-steps-title = What you will do
compat-steps-none = No steps from you.
compat-never-checked = Nobody has tested this app yet. PCTwin will try it and tell you what happened.
compat-checked-on = Checked on { $date }.
compat-version = Version tested: { $version }
work-strip = Work check
work-title = Check that you can work here
work-body = Add the files, fonts and jobs you need on this laptop. PCTwin looks at what it can read. You say whether the rest worked. Nothing is opened or run.
work-empty = No checks yet. Add a file, a font or a job below.
work-looked-at = Looked at { $checked } of { $of }.
work-missing = Missing:
work-and-more =
    { $count ->
        [one] And { $count } more.
       *[other] And { $count } more.
    }
work-not-looked-at = Not looked at:
work-answer-worked = It worked
work-answer-worked-name = It worked: { $name }
work-answer-not = It did not work
work-answer-not-name = It did not work: { $name }
work-remove = Remove
work-remove-name = Remove { $name }
work-run = Check now
work-add-title = Add a check
work-add-file = Choose a file
work-add-file-hint = A project, an archive or a database. PCTwin reads it and never changes it.
work-font-label = A font, by name
work-add-font = Add font
work-task-label = A job you do yourself
work-task-hint = For example: print a boarding pass. You say whether it worked.
work-add-task = Add job
repair-strip = Fix what did not finish
repair-title =
    { $count ->
        [one] Fix this item
       *[other] Fix these { $count } items
    }
repair-title-loading = Fix what did not finish
repair-body =
    { $kept ->
        [one] Only the items below are tried again. { $kept } item that already worked stays as it is.
       *[other] Only the items below are tried again. { $kept } items that already worked stay as they are.
    }
repair-nothing = Nothing needs fixing.
repair-can-now = Can be tried now
repair-waiting = Waiting
repair-fixed =
    { $count ->
        [one] { $count } item fixed.
       *[other] { $count } items fixed.
    }
repair-run =
    { $count ->
        [one] Try { $count } item again
       *[other] Try { $count } items again
    }
changed-strip = Changes since you approved
changed-title =
    { $count ->
        [one] { $count } thing changed after you approved
       *[other] { $count } things changed after you approved
    }
changed-body = Only these need your approval again. Everything else stays approved.
changed-affects = Affects: { $steps }
changed-held =
    { $count ->
        [one] { $count } step waits for your answer.
       *[other] { $count } steps wait for your answer.
    }
changed-leave-out = Leave these out
changed-approve = Approve again
safe-stop-strip = Update
safe-stop-title = Waiting for a safe moment
safe-stop-body = PCTwin does not update or remove itself in the middle of a move. It waits until nothing is half done.
safe-stop-failed = PCTwin could not tell whether it is safe. Try again.
safe-stop-ready = Nothing is in the middle of changing. PCTwin can update or be removed now.
safe-stop-waiting-for = It is waiting for these to finish:
safe-stop-nothing-going-on = No move, setup or undo is going on.
safe-stop-check = Check again
# Help feedback, safe stop links, catch-up and repair routes
feedback-strip = Feedback
feedback-title = How did the move go?
feedback-body = Feedback is optional and does not affect free moves.
feedback-rating-label = Rating
feedback-rating-none = No rating
feedback-rating-very-good = Very good
feedback-rating-good = Good
feedback-rating-needs-work = Needs work
feedback-words-label = What would make this better?
feedback-words-hint = Optional. Leave out names, passwords and file names.
feedback-preview-title = This is all that would be shared
feedback-preview-rating = Rating: { $rating }
feedback-preview-empty = Nothing yet. Choose a rating or write a few words.
feedback-sends-nothing = PCTwin sends nothing by itself. No account is needed. To share this, copy it, then open getpctwin.com to reach the PCTwin team.
feedback-copy = Copy feedback
feedback-copied = Feedback copied.
feedback-copy-failed = The feedback could not be copied. Select the text above and copy it yourself.
feedback-open-site = Open getpctwin.com
help-home-feedback = Feedback
help-home-update = Update or remove PCTwin
safe-stop-see = See what it is waiting for
history-repair = Fix items
kit-strip = Recovery kit
kit-title = Save a recovery kit
kit-body = A recovery kit is a small record of this move, saved in a folder you choose, for example on a USB drive. It shows what happened and what was left to do, even without this laptop.
kit-lacks-sentence = The kit does not contain your files themselves. It cannot bring back files that are missing.
kit-not-saved = No folder was chosen. Nothing was saved.
kit-saved-to = Saved to:
kit-files-title = Files in the kit
kit-holds-title = What the kit holds
kit-lacks-title = What the kit does not hold
kit-save = Choose a folder and save
kit-save-again = Save again
kit-history = Recovery kit
finish-step-repair =
    { $count ->
        [one] Fix this item
       *[other] Fix these { $count } items
    }
finish-step-repair-detail = Try again only what did not finish. What worked stays.
finish-step-work = Check that you can work here
finish-step-work-detail = Check that the files, fonts and jobs you need are here.
finish-step-kit = Save a recovery kit
finish-step-kit-detail = Keep a small record of this move in a folder you choose.
space-box-title = This laptop does not have room for everything
space-box-choice = { $count ->
        [one] Leave 1 file ({ $size }) you have not used for the longest time on your old laptop
       *[other] Leave { $count } files ({ $size }) you have not used for the longest time on your old laptop
    }
space-box-kept = Nothing is deleted. They stay on your old laptop.
space-box-short = Even then, { $size } is still short. Untick some folders, or choose another place for them.
space-box-fits = With them left on your old laptop, everything else fits.
finish-handover-check-old = Check the old laptop again
finish-handover-check-armed = PCTwin is ready to check the old laptop. Open PCTwin on the old laptop and connect the two laptops as for a move. Then choose Look again.
finish-handover-look-again = Look again
qs-where-title = Where should these go?
qs-where-body = { $count ->
        [one] 1 file, { $size }. Nothing is saved until you choose.
       *[other] { $count } files, { $size }. Nothing is saved until you choose.
    }
qs-where-folder = Folder: { $folder }
qs-where-details = More details
qs-where-path = Full place: { $path }
qs-where-save = Save here
qs-where-choose = Choose another folder
qs-where-stop = Stop

# 2.9 destination changed after approval
outcome-landed-moved = This folder was moved during the move. Your files are in it, wherever it is now.
move-place-changed-title = A folder changed after you approved
move-place-changed-body = A folder the files were going into was moved or replaced after you approved the move. Nothing was copied. Check the folder, then start the move again.

# Quick Send stage 1
qs-refusal-not-paired = The laptops are not connected any more. Connect them again.
qs-refusal-not-waiting = There is nothing waiting for this now.
qs-refusal-failed = That did not work. Try again.
qs-refusal-note-empty = Type something to send first.
qs-refusal-note-too-long = This note is too long. Keep it under 1,000 characters.
qs-refusal-note-too-many-lines = This note has too many lines. Keep it to 20 lines.
qs-refusal-note-hidden = This note has a character that cannot be shown safely. Type it again.
qs-refusal-no-link = There is no link to open.
qs-refusal-not-found = This is not there any more.
qs-refusal-restored = Your laptop was restored or copied. Connect the two again with a code.
qs-refusal-bad-name = Give it a name of up to 40 letters.
qs-refusal-store-unavailable = PCTwin could not reach your laptop's safe place for keys. Try again after signing in.
qs-refusal-too-many-connections = You have too many saved connections. Forget one first.
qs-refusal-folder-runs-at-sign-in = This folder starts programs when you sign in. Choose another folder.
qs-refusal-folder-shell-start-up = This folder runs things when a terminal opens. Choose another folder.
qs-refusal-folder-holds-keys = This folder holds keys. Choose another folder.
qs-refusal-folder-system = This is one of the system's own folders. Choose another folder.
qs-refusal-folder-not-plain = This is not a plain folder. Choose another folder.
qs-refusal-folder-not-the-same = This is not the folder you chose before. Choose it again.
qs-refusal-not-nearby = That laptop is not nearby. It may be off or on another network.
qs-refusal-over-cap = Waiting sends would use too much space. Cancel one first.
qs-refusal-no-room = There is not enough free space.
qs-refusal-bad-days = Choose from 1 to 30 days.
qs-refusal-bad-recipients = Choose each person once, up to 20.
qs-refusal-too-many-files = There are too many files. Choose fewer.
qs-refusal-changed-while-copying = A file changed while it was being copied. Try again.
qs-note-copy = Copy
qs-note-open = Open
qs-note-link-to = This link goes to { $host }.
qs-note-sent = The note arrived.
qs-remember-ask = Remember this connection?
qs-remember-body = Next time you can send without a code. It means the same laptop as before, not a person.
qs-remember-fallback = Here, the key is kept in a file in your account.
qs-known-same-device = Same device as before
qs-known-restored = Your laptop was restored or copied. Connect again.
qs-known-last-used = Last used { $date }
qs-known-forget = Forget
qs-known-forget-body = The key and every permission are deleted. Files already sent stay where they are.
qs-known-allow-folder = Allow future sends into a folder
qs-known-allow-body = Files go into a folder named PCTwin Quick Send inside it. This ends on its own after 30 days.
qs-known-wants-to-send = { $label } wants to send you this.
qs-known-delivered = Files from { $label } arrived in { $folder }.
qs-waiting-days =
    { $count ->
        [one] Waits 1 day
       *[other] Waits { $count } days
    }
qs-waiting-files =
    { $count ->
        [one] 1 file waiting
       *[other] { $count } files waiting
    }
qs-waiting-newer = A newer version exists. Review it again.
qs-waiting-on = Sends only while your laptop is on.
qs-waiting-cancel = Cancel and delete the copy

# Quick Send stage 1 fixes
qs-known-cannot-tell = PCTwin cannot tell if this is the same laptop.
qs-remember-replaces = Connected these two before? Choose the earlier connection, and it is forgotten.
qs-remember-replaces-suggested = This may be { $label } again. Replace it?
qs-remember-replace = Replace
qs-remember-keep-both = No, keep both

# 2.6 wiring
reason-open-in-app = An app had this file open on the old laptop, so PCTwin did not copy it (a copy made while the app writes could be broken). Close that app on the old laptop, then start a new move that includes this file.
report-open-in = Open in { $app }
fullmove-close-files = These files are open in an app on this laptop, so PCTwin cannot copy them safely yet. Close the app using them, then choose Try again. If you go on without them, they stay on this laptop and the report lists them.
choose-online-only =
    { NUMBER($count) ->
        [one] Also bring the file kept only online in { $provider } (1 file, { $size }). It is downloaded on this laptop first, then put back online only.
       *[other] Also bring the files kept only online in { $provider } ({ $count } files, { $size }). They are downloaded on this laptop first, then put back online only.
    }
choose-online-only-unnamed = an online storage service on this laptop
choose-online-only-metered = Also on a connection that may charge for data (a phone hotspot)
choose-online-only-not-here = If some of your files are kept only online, they stay in your online storage. PCTwin cannot bring a copy of them from this laptop yet.
kept-online-copied =
    { NUMBER($count) ->
        [one] 1 file kept only online was downloaded and copied.
       *[other] { $count } files kept only online were downloaded and copied.
    }
kept-online-safe =
    { NUMBER($count) ->
        [one] 1 file kept only online was not copied. It is still kept online: open it on the new laptop from the app that keeps it.
       *[other] { $count } files kept only online were not copied. They are still kept online: open them on the new laptop from the app that keeps them.
    }
kept-online-still-here =
    { NUMBER($count) ->
        [one] 1 downloaded file still takes { $size } on this laptop, because its cloud app has not made it online only again. To free the space, right-click it and choose Free up space.
       *[other] { $count } downloaded files still take { $size } on this laptop, because their cloud app has not made them online only again. To free the space, right-click each one and choose Free up space.
    }
kept-online-paused-metered = Downloading stopped because this connection may charge for data. The rest were not copied and are still kept online.
kept-online-paused-space = Downloading stopped because this laptop is running out of space. The rest were not copied and are still kept online.
plan-own-folder = Put the things from { $laptop } in their own folder
plan-own-folder-help = This laptop already has things from another old laptop. A folder for each laptop keeps them apart.
plan-own-folder-name = From { $laptop }
plan-own-folder-name-unnamed = From the other laptop
plan-downloads =
    { NUMBER($count) ->
        [one] 1 app is downloaded from the internet to install it on this laptop.
       *[other] { $count } apps are downloaded from the internet to install them on this laptop.
    }
plan-wait-free-wifi = Wait for free Wi-Fi before downloading
plan-wait-free-wifi-help = If the connection may charge for data when the apps are downloaded, PCTwin waits up to 20 minutes for free Wi-Fi. Apps it could not download are listed for you to install later.
plan-reuse-installers =
    { NUMBER($count) ->
        [one] Use the installer found on the old laptop for 1 app, instead of downloading it
       *[other] Use the installers found on the old laptop for { $count } apps, instead of downloading them
    }
plan-reuse-installers-help = Each one is checked first, the same way as a download: it must be signed by the app's own maker.
fullmove-close-files-title = Close the apps using these files
choose-online-only-title = Files kept only online
kept-online-title = Files kept only online
kept-online-downloaded = { $size } downloaded
plan-own-folder-called = The folder is called { $folder }.
accept-own-folder = The things from the other laptop go in their own folder, { $folder }. You can turn this off in the details.
plan-downloads-title = Apps from the internet
plan-connection-free = This laptop's connection does not charge for data now.
plan-connection-may-charge = This laptop's connection may charge for data now, such as a phone hotspot.
plan-connection-not-known = PCTwin cannot tell whether this laptop's connection charges for data.
fullmove-rebuilding-wait-wifi = If this connection may charge for data, PCTwin waits up to 20 minutes for free Wi-Fi before it downloads apps. Connect to free Wi-Fi to carry on sooner.

# Quick Send stage 1 screens
qs-note-card-title = Send a note or link
qs-note-card-body = A few lines of text, or one web link.
qs-home-known = Remembered connections
qs-home-waiting = Waiting sends
qs-note-title = Send a note or link
qs-note-intro = Type a few lines, or paste one web link. Only what you type here is sent.
qs-note-label = Note or link
qs-note-hint = Up to 20 lines and 1,000 characters.
qs-note-never-opens = A link you send opens only if the person who gets it chooses Open.
qs-note-sending-title = Sending the note
qs-note-sending-body = Please wait while the receiving laptop takes it.
qs-note-sent-title = The note arrived
qs-note-sent-body = The receiving laptop has it. PCTwin cannot tell if it was read.
qs-note-received-title = A note arrived
qs-note-received-link-title = A link arrived
qs-note-from = From { $label }.
qs-note-from-code = From the laptop you connected with a code.
qs-note-link-safety = It opens in your web browser only if you choose Open.
qs-note-copied = Copied.
qs-note-copy-failed = PCTwin could not copy it. Select the text and copy it yourself.
qs-remember-name = Name for this connection
qs-remember-name-hint = Up to 40 letters, so you can find it later.
qs-remember-save = Remember
qs-remember-done = Remembered as { $label }. Files from it still ask you first.
qs-known-title = Remembered connections
qs-known-intro = Laptops you can send to without a code. Each one means the same laptop as before, not a person.
qs-known-empty = No connections are remembered yet. After you send or receive with a code, PCTwin asks if you want to remember that laptop.
qs-known-expires = Forgotten on its own on { $date } if not used before then.
qs-known-rename = Rename
qs-known-rename-label = New name
qs-known-rename-save = Save name
qs-known-forget-title = Forget { $label }?
qs-known-forget-confirm = Forget it
qs-known-folder-on = Files from it go into { $folder }, in a folder named PCTwin Quick Send.
qs-known-folder-ends = This ends on { $date }.
qs-known-folder-ended = This ended on { $date }. Files from it ask you first again.
qs-known-renew = Renew for 30 days
qs-known-next-title = Folder for next time
qs-known-next-on = { $folder } is chosen already when it sends you files. You can still change it each time.
qs-known-next-none = Not set. When it sends you files, Downloads is chosen already, or you choose another folder each time.
qs-known-next-use = Use the last folder
qs-known-next-clear = Stop using it
qs-known-in-more = and { $count } more
qs-known-in-next = Use this folder for { $label } next time
qs-known-in-decline = Decline
qs-known-delivered-counts = { $arrived } of { $total } copied and checked.
qs-send-to = Send to { $name }
qs-to-title = Who should get these?
qs-to-intro = Choose laptops you remembered, or connect a laptop with a code.
qs-to-known = Remembered laptops
qs-to-code = Use a code instead
qs-to-when = When
qs-to-now = Now
qs-to-now-hint = The receiving laptop must be on and nearby.
qs-to-later = When it is nearby
qs-to-several = Each laptop gets them when it is nearby.
qs-to-days = How long to wait
qs-to-each-asks = The receiving laptop is asked where to save, unless it allowed sends into a folder.
qs-to-approve-hint = You approve these files as they are now, these laptops and this time. PCTwin keeps a copy of the files to send. Each laptop is asked where to save, unless it allowed sends into a folder.
qs-to-send-now = Send now
qs-to-approve = Approve this delivery
qs-waiting-title = Waiting sends
qs-waiting-intro = Files PCTwin keeps a copy of, to send when each laptop is nearby.
qs-waiting-empty = Nothing is waiting.
qs-waiting-approved = Approved on { $date }
qs-waiting-until = Waits until { $date }
qs-waiting-ended = Ended on { $date }. Nothing more is sent.
qs-waiting-to-unknown = A forgotten connection
qs-waiting-state-waiting = Waiting
qs-waiting-state-delivered = { $arrived } of { $total } copied and checked
qs-waiting-state-needs = Needs action: { $count }
qs-waiting-state-declined = Declined on { $date }
qs-waiting-state-failed = Did not copy: { $count }
qs-waiting-renew-label = How long to wait from now
qs-waiting-renew = Renew
qs-waiting-cancel-title = Cancel this waiting send?
qs-waiting-cancel-body = PCTwin deletes its copy, and nothing more is sent. Laptops that already have the files keep them. Your own files stay as they are.
qs-waiting-keep = Keep it

# Quick Send more
qs-session-add-files = Add files
qs-session-send-to = Send to { $name }
qs-session-send-back = Send files back
qs-session-end = End
qs-session-ended = This send is over. Nothing more arrives.
qs-sends-waiting = Waiting its turn
qs-sends-sending = Sending
qs-sends-paused = Paused
qs-send-pause = Pause
qs-send-resume = Resume
qs-send-cancel = Cancel

# Open sends: Add files, Send to, several laptops at once, questions that wait, a live file being copied
qs-session-ended-name = { $name }: this send is over. Nothing more arrives.
qs-sends-action-name = { $what }: { $name }
qs-sends-title = Your sends
qs-sends-intro = PCTwin sends to up to three laptops at the same time, and one send after another to the same laptop. The others wait their turn.
qs-sends-empty = Nothing is being sent now.
qs-sends-ended-title = Finished sends
qs-sends-stopped = It was stopped before the end.
qs-sends-result-whole = Copied and checked
qs-sends-result-partial = Some did not copy
qs-sends-not-sent = Not sent to { $name }.
qs-sends-failed = PCTwin could not change that send. Try again.
qs-session-offer-waiting = { $name } is sending you more files.
qs-session-offer-show = Choose where to save
qs-to-now-several-hint = Each laptop must be on and nearby. PCTwin sends to up to three at the same time; the others wait their turn.
qs-asking-files = { $label } wants to send you files.
qs-asking-note = { $label } sent you a note.
qs-asking-later = PCTwin asks you about it when you leave this page.
qs-asking-show = Choose where to save
qs-asking-show-note = Show the note
progress-current-open-in = While PCTwin copies { $name }, { $app } cannot save changes to it. If { $app } says the file is in use, wait for the copy to finish.

# 2.6 waiting for free Wi-Fi
fullmove-rebuilding-waiting-wifi = { $minutes ->
        [one] Waiting for free Wi-Fi before downloading the apps: up to 1 more minute
       *[other] Waiting for free Wi-Fi before downloading the apps: up to { $minutes } more minutes
    }

# 2.5 files held by an app nobody can name
fullmove-live-wait-title = Waiting for files to be free
fullmove-live-wait-body = These files are open in an app on this laptop. PCTwin checks again every 30 seconds, for up to { $minutes } more minutes. If they are still open then, the move goes on without them and the report lists them.
fullmove-live-wait-stop = Stop the move

# 2.5 games
special-note-saves-in-launcher-cloud = Your game launcher keeps these saves online. They come back when you sign in to it on your new laptop, so PCTwin does not copy them unless you tick them. The old laptop's copy stays as it is.
special-note-save-may-not-work-here = These saves come from a different kind of system. PCTwin keeps them on your new laptop, but the game there may not be able to open them.
special-note-store-protected-saves = The store keeps these saves for itself, so PCTwin cannot copy them. They come back when you sign in to the store on your new laptop and install the game.
special-note-mod-programs-held = Some of this game's add-ons are programs. PCTwin lists them but does not copy them or turn them on. Get them again from their maker.
special-note-mod-programs-as-files = Some of this game's add-ons are programs. They move with your other files as plain files, and PCTwin never turns them on. Get working copies from their maker.
special-note-launcher-verify = Install the launcher on your new laptop and sign in. Then use its own verify or locate step so it finds and checks your games. PCTwin copies installed games only if you choose to.
choose-wifi-title = Saved Wi-Fi networks
choose-wifi-admin = Ticked networks move with their passwords, so the new laptop joins them by itself. To read the passwords, this laptop may ask for an administrator’s approval in its own window.
choose-wifi-ask-admin = Reading Wi-Fi passwords needs an administrator of this laptop, so none is ticked. To bring them, ask whoever looks after this laptop, then tick them: this laptop asks for their approval in its own window.
choose-wifi-open = No password
choose-wifi-organisation = Set up with an organisation: join it again on the new laptop with their details
choose-wifi-other = PCTwin does not move this kind of network: join it again on the new laptop
move-wait-wifi-title = Reading your saved Wi-Fi passwords
move-wait-wifi-body = This laptop may ask for an administrator’s approval in its own window. Answer it to bring the Wi-Fi networks you ticked. Nothing on this laptop is changed.
fullmove-group-wifi = Saved Wi-Fi networks: { $count }
fullmove-wifi-step-add = PCTwin adds it here with its password, which is sent encrypted after you approve.
fullmove-wifi-step-add-by-profile = You add it here once, in System Settings, for all these networks. PCTwin opens the page.
fullmove-wifi-step-skipped = Not ticked on the old laptop, so it stays there.
fullmove-wifi-step-organisation = Set up with an organisation: join it again here with their details.
fullmove-wifi-step-not-this-kind = PCTwin does not move this kind of network: join it again here.
fullmove-wifi-step-no-password = Its password could not be read on the old laptop: type it once when you join this network here.
fullmove-wifi-why-organisation = Set up with an organisation: join it again with their details.
fullmove-wifi-why-not-this-kind = PCTwin does not move this kind of network: join it again here.
fullmove-wifi-why-no-password = Its password could not be read on the old laptop: type it once when you join this network.
fullmove-wifi-why-did-not-arrive = It did not arrive from the old laptop: type its password once when you join this network.
fullmove-wifi-why-already-here = A network with this name was already saved here, so PCTwin left it as it is. If it does not join, type its password once.
fullmove-wifi-why-not-added = This laptop did not add it: type its password once when you join this network.
fullmove-wifi-why-install-profile = Not added: the profile “Wi-Fi networks from your old laptop (PCTwin)” was not installed in System Settings in time. Type its password once when you join this network.
fullmove-rebuilding-wifi = Adding your saved Wi-Fi networks
fullmove-rebuilding-wifi-profile = { $minutes ->
        [one] To add your Wi-Fi networks, install the profile “Wi-Fi networks from your old laptop (PCTwin)” on the System Settings page PCTwin opened: up to 1 more minute
       *[other] To add your Wi-Fi networks, install the profile “Wi-Fi networks from your old laptop (PCTwin)” on the System Settings page PCTwin opened: up to { $minutes } more minutes
    }
report-group-wifi = Wi-Fi networks
report-count-wifi = { $count ->
        [one] 1 network
       *[other] { $count } networks
    }
