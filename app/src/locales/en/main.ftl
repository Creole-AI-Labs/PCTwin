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

connect-eyebrow = Local move
connect-title = How will you connect?
connect-body = Use the same guided plan on every supported transport.
connect-wifi-title = Wi-Fi
connect-wifi-body = Release 1 · same local network.
connect-wifi-status = Available
connect-later = Ethernet or direct link, the PCTwin bridge cable and the encrypted drive archive are planned for Release 2.
connect-usb-note = An ordinary USB cable does not create a supported host-to-host link.
connect-use-wifi = Use Wi-Fi
# Added: sizes and speeds as measured, shown the same way on every connection screen.
connect-unit-mbps = { NUMBER($value, maximumFractionDigits: 0) } MB/s
connect-unit-gb = { NUMBER($value, maximumFractionDigits: 1) } GB

## C02 Put both laptops together

connect-prep-title = Put both laptops together.
connect-prep-body = Connect them to the same local network, then open PCTwin on each.
connect-prep-step-1-title = Join the same Wi-Fi
connect-prep-step-1-body = Internet is not needed for the LAN copy.
# $network is the Wi-Fi name this laptop is on.
connect-prep-step-1-network = Network: { $network }.
connect-prep-step-2-title = Allow local-network access
connect-prep-step-2-body = Use the genuine OS prompt if it appears.
connect-prep-step-3-title = Choose a role on each laptop
connect-prep-step-3-body = One sends. The other receives.
connect-prep-note = Guest Wi-Fi or client isolation can block the link.
connect-prep-continue = Continue to pairing

## C06 Meet your other laptop

peer-eyebrow = Local move
peer-title = Meet your other laptop.
peer-body = The confirmed pair is ready for a supported move.
peer-sending = Sending
peer-receiving = Receiving
peer-old = Old laptop
peer-new = New laptop
peer-name-label = Name
peer-system-label = System
peer-version-label = PCTwin
# $version is the PCTwin version on that laptop, for example "1.0.3".
peer-version = { $version }
peer-status-ready = Ready
peer-status-update = Blocked
peer-status-unsupported = Waiting
peer-ready-note = Authenticated pairing comes before inventory. Unsupported versions need a trusted update.
peer-version-differs = This pair needs a compatible signed build before the move can continue.
peer-version-old-body = Incompatible protocol build: { $version }.
peer-version-new-body = Needs a matching supported peer.
peer-update-title = Trusted update
peer-update-body = Use the compatible signed release.
# $from and $to are system names, for example "Windows 11" and "macOS". The mockup sentence does not use them.
peer-direction-unsupported = This platform direction is not supported by the selected release.
peer-continue = Assess the connection
peer-pair-again = Pair again
peer-home = Return home

## C07 Let’s check the route

check-eyebrow = Local move
check-title = Let’s check the route.
check-body = Measured resources guide the plan and the time estimate.
check-measuring = Measuring the connection…
# The value next to it is already formatted, for example "62 MB/s".
check-speed-label = local link
check-space-label = free target space
check-link-title = Connection
check-link-good = Strong. Files should move steadily.
check-link-weak = Weak, so moving may be slow. Moving the laptops closer to the Wi-Fi router can help.
check-power-title = Power
check-power-both = Both laptops connected.
check-power-battery = { $label } is running on battery. Plug it in so a long move is not interrupted.
check-space-title = Storage
check-space-body = { $free } free on the new laptop. Before anything moves, you will see whether your choices fit.
check-internet-title = Internet
check-internet-on = Needed for fresh app downloads, not this LAN file copy.
check-internet-off = Not connected. That is fine: copying files does not need the internet.
check-status-ready = Ready
check-status-slow = Slow
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
reconnect-body = The last committed blocks are saved.
# Added: the title of the saved-progress card (the mockup shows Move paused by connection).
reconnect-card-title = Move paused by connection
reconnect-progress =
    .aria-label = Move paused by connection
# $done is an already formatted size, for example "81.6 GB". $total is passed but not shown.
reconnect-saved = Checkpoint: { $done } verified or committed; pending work remains.
# $ago is already formatted, for example "45 seconds ago".
reconnect-last-seen = Last seen { $ago }.
reconnect-status-offline = Offline
reconnect-status-trying = Reconnecting
reconnect-status-pending = Pending
reconnect-trying = Trying to reconnect by itself… Keep both laptops on, awake and on the same Wi-Fi.
reconnect-expired = An expired ticket requires a fresh pairing and reconciliation.
reconnect-unavailable = { $label } cannot be reached. Check that it is on, awake, has PCTwin open and is on the same Wi-Fi.
reconnect-try-now = Retry connection
reconnect-pair-again = Pair again
reconnect-details = Resume security
reconnect-details-body = Fresh traffic keys with a valid authenticated ticket.

## C10 The connection has ended (dialog)

ended-title = The connection has ended.
ended-body = Your saved job stays available. The old code cannot be reused.
ended-kept-title = Retained
ended-kept-body = Committed move journal and verified results.
ended-not-kept-title = Not retained
ended-not-kept-body = Permission to continue with an unconfirmed peer.
ended-pair-again = Pair again
ended-pair-again-body = Start a fresh session and reconcile the job.
ended-see-moved = Open the saved job
ended-see-moved-body = Review what has already committed.
ended-home = Return home

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

## M06 Match people to their profiles (Who’s moving?)

people-eyebrow = Reviewed move
people-title = Match people to their profiles.
people-body = Every source profile needs the right owner and destination.
people-col-old = Source
people-col-new = Target account
people-col-status = Authority
people-you = You
# $name is the person's name on the old laptop. $account is an account name on the new laptop.
people-select =
    .aria-label = Target account for { $name }
people-option-existing = { $account } / new laptop
people-option-new = Create a new account
people-option-leave-out = Leave out
people-option-choose = Choose…
people-new-account-hint = The new laptop asks for permission before it creates an account.
people-suggested = Suggested because the names match. Check it is the same person.
# $account is an account name on the new laptop.
people-suggested-use = Use { $account }
people-status-ready = Confirmed
people-status-waiting = Owner permission needed
people-status-left-out = Not selected
people-status-choose = Needs your choice
people-status-changed = Changed by you
people-waiting-body = { $name } must sign in on the old laptop and approve before their files can move. Until then, their files stay untouched.
people-just-me = Only your own profile was found on the old laptop.
people-changed-note = You changed a target account. Check it before you continue.
people-shared-title = Shared items
people-shared-body = Shared folders and public documents that belong to everyone on the old laptop.
people-shared-select =
    .aria-label = Where shared items go
people-shared-option-shared = Shared folders on the new laptop
people-shared-option-person = With the files of { $name }
people-shared-option-leave-out = Leave out
people-note = PCTwin does not change anyone’s sign-in or bypass another profile’s protection.
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

before-eyebrow = Reviewed move
before-title = Before you leave this laptop.
before-body = Check the work and recovery methods you may still need.
before-licences-title = App activations
before-licences-body = Some apps need owner-managed licence handover.
before-passwords-title = Recovery methods
before-passwords-body = Confirm drive and account recovery are available.
before-codes-title = Keep your sign-in codes to hand
before-codes-body = If an account sends you a code or uses an app to confirm it is you, keep that phone or app nearby.
before-online-title = Unsynced work
before-online-body = Save drafts and check cloud originals.
before-source-title = Source-only items
before-source-body = Unresolved items stay on this laptop.
before-status-needs-action = Needs action
before-status-review = Review
before-status-keep = Keep
before-done =
    .aria-label = I have done this: { $item }
before-done-label = Done
before-ticked-count = { $done } of { $total } ticked by you
before-note = Do not enter recovery secrets here. This checklist does not authorize erasing the laptop.
before-empty = Nothing needs doing on your old laptop before the move.
before-continue = Return to the move plan

## S11 Get help with this task (a short problem code)

help-eyebrow = Help and privacy
help-title = Get help with this task.
help-body = Share a short problem code after checking what it reveals.
help-making = Making a code…
help-code-title = Problem code
# $code is the code itself, for example "PCT-7F3K-21". Keep it unchanged.
help-code =
    .aria-label = Problem code { $code }
help-contains-title = Shared detail
help-contains-problem = Problem category and approved task status.
help-contains-step = The step of the move where it happened.
help-contains-versions = The PCTwin version, and which system each laptop runs, such as Windows or macOS.
help-never-title = Private material
help-chip-included = Included
help-chip-excluded = Never included
help-never-names = Your name or anyone else’s.
help-never-files = No file contents, secrets or control grant.
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
progress-route-old = Old laptop
progress-route-new = New laptop
progress-panel-pausing = Pausing
outcome-metric-size = verified copies
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
qs-eyebrow-receiving = Receiving on this device
# The route between the two laptops, for example "Blue Fox" to "Green Owl".
qs-route =
    .aria-label = From { $from } to { $to }
qs-route-sender = Sender · this approved session
qs-route-receiver = Receiver · same usable Wi-Fi
qs-tally =
    { NUMBER($count) ->
        [one] { $count } selected item · { $size }
       *[other] { $count } selected items · { $size }
    }
qs-tally-badge = Local send
qs-back = Back

## Q01 Quick Send home

qs-home-title = A few files. Straight across.
qs-home-body = Send selected work to a nearby device.
qs-send-title = Send files
qs-send-body = Choose files or folders on this laptop.
qs-receive-title = Receive files
qs-receive-body = Turn on receiving for this session.
qs-history-title = Local handover receipts
qs-history-body = Find received locations and unfinished items.
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
qs-online-only = Cloud-only files need an available original before they can be sent.
qs-protected = PCTwin cannot read { $name }. Allow access, or remove it from the list.
qs-allow = Allow access
qs-choose-continue = Review selected files

## Q03 Check what you are sharing

qs-check-title = Check what you are sharing.
qs-check-body = Review the selected material before it leaves this device.
qs-check-included = Included
qs-check-shared-note = Selected filenames and ordinary file metadata are shared with the approved recipient.
qs-check-changed = { $name } changed since you chose it. Check it again.
qs-check-ack = I have reviewed the selected material
qs-check-ack-hint = Remove anything private before sending.
qs-check-continue = Choose a receiving device

## Q04 Who should receive it?

qs-who-title = Who should receive it?
qs-who-body = Choose the session label shown on the other device.
qs-who-nearby = Nearby receiver · same Wi-Fi.
qs-who-code = Use a pairing code
qs-who-code-body = Enter the code shown on the sender.
qs-who-same-label = Two laptops nearby have this name. Check the name on the other laptop’s screen.
qs-who-busy = { $label } is busy with another send. Wait a moment, then try again.
qs-who-note = The receiving device approves the offer and chooses its destination.
qs-who-action = Show my send code

## Q08 Incoming offer (receiving laptop)

qs-offer-title = Someone wants to send you files.
qs-offer-body = Review this offer on the receiving device.
qs-offer-note = Receiving saves approved copies. It does not open or run them.
qs-offer-accept = Accept offer and choose a folder
qs-offer-decline = Decline this offer
qs-offer-declined = You declined. Nothing was saved on this laptop.
qs-offer-changed = The sender changed the files. Check the new list before you accept.
qs-offer-gone = { $label } is no longer available. Nothing was saved on this laptop.

## Q09 Where should the files land?

qs-land-title = Where should the files land?
qs-land-body = You choose the receiving folder on this device.
qs-land-folder-label = Receiving folder
qs-land-choose = Another approved folder
qs-land-space-title = Space available
qs-land-space = { $free } available · { $needed } needed.
qs-land-status-fits = Fits
qs-land-status-no-space = Not enough space
qs-land-no-space = This folder has { $free } free, but these files need { $needed }. Choose another folder or free up space.
qs-land-unavailable = This folder is not available. Connect its drive, or choose another folder.
qs-land-denied = PCTwin is not allowed to save in this folder. Choose another folder.
qs-land-note = If files already exist, you review the conflict choices before writing.
qs-land-confirm = Use this folder for this offer
qs-land-confirm-hint = Only the reviewed files may be saved here.
qs-land-use = Use this folder

## Q10 Keep your existing work safe

qs-conflict-title = Keep your existing work safe.
qs-conflict-body = Choose what happens when a filename already exists.
qs-conflict-col-file = Item
qs-conflict-col-existing = Existing copy
qs-conflict-col-choice = Choice
# $when is already formatted, for example "yesterday".
qs-conflict-existing = Edited { $when }
qs-conflict-none = No existing copy
qs-conflict-select =
    .aria-label = What happens to { $name }
qs-keep-both = Keep both
qs-skip = Skip this item
qs-replace = Replace after separate review
qs-add-new = Add new
qs-keep-both-note = Keep both preserves the existing file. The received copy is saved as { $newName }.
qs-no-conflicts = None of these files are in the folder yet. Nothing will be replaced.
qs-replace-separate = Replacing needs its own confirmation, separate from the other choices.
qs-replace-warning = Replacing removes the existing target version from this location. Review that exact file separately.
qs-replace-confirm = I separately approve replacing { $name }
qs-replace-confirm-hint = Its current identity and write state must still be checked.
qs-conflict-target-changed = A changed or inaccessible target stays unresolved until it is reviewed.
qs-conflict-approve = I approve these receiving choices
qs-conflict-continue = Confirm receiving choices

## Q11 Ready to send this selection?

qs-approve-title = Ready to send this selection?
qs-approve-body = The sender reviews the exact handover before starting.
qs-approve-receiver-title = Recipient
qs-approve-receiver-waiting = Waiting for the receiving device’s approved offer and folder.
qs-approve-receiver-ready = The receiver approved this offer, folder and conflict choices.
qs-approve-files-title = Files
qs-approve-files-same = Selected source versions need to remain unchanged.
qs-approve-files-changed = Some files changed since you checked them. Check them again.
qs-approve-existing-title = Existing files
qs-approve-existing-body = Keep both where reviewed.
qs-status-waiting = Pending
qs-status-accepted = Approved
qs-status-unchanged = Review
qs-status-changed = Changed
qs-approve-ack = Send this reviewed selection
qs-approve-ack-hint = I approve these files and this recipient.
qs-approve-start = Start approved send
qs-approve-cleared = You changed the files, so { $label } must accept them again.
qs-approve-withdrawn = { $label } no longer accepts this send. Nothing was sent.

## Q12 Send progress

qs-sending-title = Your files are on their way.
qs-sending-body = Selected copies travel directly between these devices.
qs-sending-progress =
    .aria-label = Sending selected files
qs-sending-current = Now sending: { $name }
# $speed is measured and already formatted, for example "3.1 MB/s".
qs-sending-speed = observed { $speed }
qs-step-copying = Copying
qs-step-copying-body =
    { NUMBER($done) ->
        [one] { $done } selected item received.
       *[other] { $done } selected items received.
    }
qs-step-checking = Checking
qs-step-checking-body = Completed files are checked individually.
qs-step-receipt = Receipt
qs-step-receipt-body = Locations and unresolved items appear at the end.
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
qs-sending-complete-title = The selected handover is complete
qs-sending-complete-body = Copy checks finished. Review the actual receiving locations.
qs-sending-receipt = View current receipt

## Q14 Handover receipt (both laptops)

qs-receipt-title = Here is your handover receipt.
qs-receipt-body = See exactly what arrived and where it landed.
qs-receipt-box-title = Your files arrived
qs-receipt-box-body = The selected copies were checked.
qs-receipt-arrived =
    { NUMBER($count) ->
        [one] { $count } selected item received
       *[other] { $count } selected items received
    }
qs-receipt-size = { $size } checked against the source
qs-receipt-saved-as = { $path }
qs-receipt-not-arrived = The complete folder has not been verified.
qs-receipt-status-verified = Verified
qs-receipt-status-unfinished = Unfinished
qs-receipt-not-completed = Not completed.
qs-receipt-stopped-row = Received and checked before stopping.
qs-receipt-partial =
    { NUMBER($count) ->
        [one] { $count } item still needs attention.
       *[other] { $count } items still need attention.
    }
qs-receipt-stopped = This send was stopped. Complete received copies remain in their approved folder.
qs-receipt-not-opened = PCTwin did not open or run any received file.
qs-receipt-ack = Acknowledge receipt
# $when is already formatted, for example "today at 14:05".
qs-receipt-acked = Receipt acknowledged { $when }.
qs-receipt-ack-note = Acknowledging this receipt does not confirm the files were opened or used.
qs-receipt-offline = { $label } is offline, so it has not seen this receipt yet. The receipt is saved on this laptop.
qs-receipt-missing = Send missing pieces
qs-receipt-missing-body = Review only the affected selected files.
qs-receipt-history = All local receipts
qs-receipt-done = Back to Quick Send

## Q19 Finish the missing part

qs-missing-title = Finish the missing part.
qs-missing-body = Use the prior receipt to review unfinished work.
qs-missing-status-there = Keep
qs-missing-status-missing = Needs action
qs-missing-there-body = The received copy was checked.
qs-missing-missing-body = Review the missing or incomplete selected material.
qs-missing-note = Completed files are preserved. New or changed material needs its own approval.
qs-missing-ack = Send the missing selected material
qs-missing-ack-hint = Recheck current source versions and receiving approval.
qs-missing-complete = Everything already arrived. There is nothing left to send.
qs-missing-unavailable = The earlier receipt cannot be found, so PCTwin cannot tell what is missing. Start a new send instead.
qs-missing-continue = Review this focused send

## Q20 Stop this handover? (and decline)

qs-stop-title = Stop this handover?
qs-stop-body = Review what has already arrived before stopping.
qs-stop-arrived-title = Complete received copies
qs-stop-arrived-body = Stay in the approved receiving folder.
qs-stop-rest-title = Pending work
qs-stop-rest-body = No new transfer work starts after stopping.
qs-stop-partial-title = Partial temporary files
qs-stop-partial-body = Reconciled and cleaned under the task’s policy.
qs-stop-status-keep = Keep
qs-stop-status-stop = Stop
qs-stop-status-review = Review
qs-stop-ack = Stop this named send
qs-stop-ack-hint = This does not delete complete received copies.
qs-stop-confirm = Stop and view the receipt
qs-stop-keep = Keep sending
qs-decline-title = Decline these files?
qs-decline-body = Nothing is saved on this laptop, and the sender is told you declined.
qs-decline-confirm = Decline
qs-already-finished = This send has already finished. See the receipt for what arrived.

## F08 Your moves, kept locally (history)

history-title = Your moves, kept locally.
history-body = Reopen a report or resume a journalled job.
history-row-move = { $from } → { $to }
history-row-sent = Sent to { $label }
history-row-received = Received from { $label }
history-status-finished = Completed
history-status-steps-left =
    { NUMBER($count) ->
        [one] Completed with { $count } manual step
       *[other] Completed with { $count } manual steps
    }
history-status-paused = Paused
history-status-stopped = Cancelled safely
history-status-partial = Not all arrived
history-open-report = Report
history-open-receipt = Receipt
history-continue = Continue
history-export = Export history
history-exported = History saved as { $path }.
history-export-failed = The history could not be saved there. Choose another folder and try again.
history-note = This history stays on this laptop. Nothing is sent anywhere unless you export it and share it yourself.
history-empty = Your first local journal and report will appear here.
history-empty-title = No moves recorded yet
history-start = Start a new move
history-receipts-title = Quick Send receipts
history-receipts-body = Find received locations and unfinished selected items.

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
qs-footer-free = Free nearby sending
qs-land-none = No folder chosen yet.
qs-land-space-unknown = PCTwin could not measure the free space here. These files need { $needed }.
qs-conflict-existing-unknown = Already there
qs-conflict-choice-open = Not chosen yet
qs-conflict-why = Choose what happens to every file, then tick the box, to continue.
qs-approve-folder-title = Receiving folder
qs-status-recipient-choice = Recipient choice
qs-step-done = Done
qs-step-current = Now
qs-step-todo = Next
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
