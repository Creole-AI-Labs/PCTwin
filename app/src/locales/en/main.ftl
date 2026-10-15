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
