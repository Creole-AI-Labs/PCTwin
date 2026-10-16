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
peer-version-label = PCTwin
# $version is the PCTwin version on that laptop, for example "1.0.3".
peer-version = { $version }
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

## C07 Let’s check the route

check-eyebrow = Local move
check-title = Let’s check the route.
check-body = Measured resources guide the plan and the time estimate.
check-measuring = Measuring the connection…
# The value next to it is already formatted, for example "62 MB/s".
check-speed-label = local link
check-space-label = free target space
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
before-done =
    .aria-label = I have done this: { $item }
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
