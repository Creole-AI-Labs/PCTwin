# Word changes, 29 October 2026

Why: the founder asked for plain, specific, calm words with no slogans or AI-sounding lines, on every screen.
The rules are in `STYLE.md`. Every changed key keeps its id, placeholders, plural forms and markup. The other six
languages were machine-translated ahead of time for every changed key (no live translation).

Areas with no rows below (welcome, settings) were read line by line and already met the rules.

Research note: Apple's Human Interface Guidelines (Writing) and Google Material 3 (Content design) pages are drawn by
script and could not be read by the research tool this session, so they are not cited; every source below was read.

Totals: 82 keys reworded, plus 46 headings that only lost their final full stop: 128 keys in all.

## Home

Sources and the pattern taken from each:

- **Shopify Polaris, Button content guidelines (polaris.shopify.com/components/button)**: {verb}+{noun} labels so the person can predict what happens: "Create order", not "New order".
- **Microsoft Writing Style Guide, Top 10 tips (learn.microsoft.com/style-guide/top-10-tips-style-voice)**: "Use bigger ideas, fewer words" and "Get to the point fast": card lines name the job, not a mood.
- **Samsung Smart Switch guides (samsung.com/ae/support, AT&T device support)**: The two jobs are plain verbs on each device: Send on the old phone, Receive on the new one.

| Key | Old | New | Why |
|---|---|---|---|
| `home-restore-body` | Bring it back | Rebuild from a saved setup | Slogan; say what the card does (Polaris verb + noun) |
| `home-rescue-body` | Save what matters | Copy files from a laptop that won’t start | Slogan; name the real situation |

## Settings and help

Sources and the pattern taken from each:

- **Mailchimp Content Style Guide, TL;DR (styleguide.mailchimp.com/tldr)**: Short sentences, familiar words, clarity before entertainment.
- **Microsoft Writing Style Guide, Top 10 tips**: Lead with what is most important; front-load the words people scan for.

| Key | Old | New | Why |
|---|---|---|---|
| `help-home-title` | Help for the task in front of you. | Common problems and what to do | Vague; say what the page holds |

## Pairing

Sources and the pattern taken from each:

- **Apple Support, Transfer to a new Mac with Migration Assistant (support.apple.com/102613)**: Check that the code on the old Mac matches the new one, then Continue: one instruction per step, standard Continue button.
- **Microsoft Support, Transfer your files and settings to a new Windows PC**: Each step says exactly what to enter on which PC (the PC name, then the code), with no chatter.
- **Google Android Help, Quick Share (support.google.com/android/answer/9286773)**: Plain state words for the other side: "Wait for the other device to accept".
- **Microsoft Writing Style Guide, Top 10 tips**: No full stop at the end of titles and headings.

| Key | Old | New | Why |
|---|---|---|---|
| `peer-title` | Meet your other laptop. | Check both laptops | "Meet" personifies the laptop; say what to do |
| `check-title` | Let’s check the connection. | Check the connection | "Let’s" chatter; title matches the button that opens it |
| `pair-old-title` | Let’s connect your laptops. | Connect your laptops | "Let’s" chatter; plain instruction |
| `pair-old-wait-title` | A new code is on its way. | Wait for a new code | Idiom "on its way"; say what to do |
| `pair-done-eyebrow` | All set | Paired | "All set" is cheer; state the fact |

## Choosing what to move

Sources and the pattern taken from each:

- **Google Android Help, Quick Share**: The receiving side gets a direct question with two buttons, Accept or Decline. Used for accept-title.
- **Samsung Smart Switch guides**: "Select the content you would like to transfer, then Send": the screen names the choice, not a feeling.
- **Shopify Polaris, Button**: "Clear and predictable... Never deceive by mislabeling": headings name the decision.

| Key | Old | New | Why |
|---|---|---|---|
| `scan-title` | Finding your familiar setup. | Finding your files | "Familiar setup" is soft-sell; say what is happening |
| `overview-title` | Everything you chose, in view. | What PCTwin found | Slogan-like; say what the screen shows |
| `files-title` | Keep what matters to you. | Choose the folders to move | "What matters to you" is a slogan; name the decision |
| `dest-title` | Give every file a home. | Where your folders go | Metaphor "a home"; the screen sets each folder’s place |
| `move-wait-list-body` | Your old laptop is finding your things, then its person chooses what moves. When that is done, what is coming appears here for you to accept. This can take a few minutes. | Your old laptop is finding your things. Then what moves is chosen there. When that is done, the list appears here for you to accept. This can take a few minutes. | "its person chooses" was unclear |
| `accept-title` | { $from } wants to send you this. | Accept the move from { $from }? | Founder rejected "<name> wants to send you this"; the heading is the decision (Quick Share: Accept or Decline) |
| `accept-apps` | { $count -> [one] 1 app, installed for you *[other] { $count } apps, installed for you } | { $count -> [one] 1 app to install *[other] { $count } apps to install } | "installed for you" is service-speak; state the count |

## Plan and approval

Sources and the pattern taken from each:

- **Microsoft Windows UX guidelines, Error messages (learn.microsoft.com/windows/win32/uxguide/mess-error)**: "Remove redundant text" in titles and buttons; avoid "please"; never phrase it as the person's fault.
- **Microsoft Writing Style Guide, Top 10 tips**: "Start each statement with a verb" and cut weak phrasing. Approve is the one verb for agreeing (no "say yes").
- **GOV.UK Design System, Error message (design-system.service.gov.uk/components/error-message)**: Words must say what is wrong and how to fix it; vague or cute words do not.

| Key | Old | New | Why |
|---|---|---|---|
| `plan-title` | Approve the move you want. | Check and approve the plan | "the move you want" is vague; name both actions |
| `plan-preview-first` | We’ll move these first, so your new laptop is ready to use sooner: { $places }. | These move first, so you can start using your new laptop sooner: { $places }. | Voice is "PCTwin", never "we" |
| `guard-changes-title` | Before we change anything | What the move will change | Voice is "PCTwin", never "we"; heading says what the list is |
| `guard-changed-body` | Only these need a fresh look. Everything else stays approved and carries on. | Only these need checking again. Everything else stays approved and continues. | Idiom "a fresh look" |
| `guard-offload-ask` | PCTwin can put some things on this drive. It needs your yes for this drive on its own. | PCTwin can put some things on this drive. It needs your approval for this drive separately. | "your yes" is cute; use the product verb approve |
| `guard-local-how` | PCTwin brings them down a few at a time, sends them, then makes them online only again on the old laptop. | PCTwin downloads them a few at a time on the old laptop, sends them, then sets them back to online only there. | "brings them down" is an idiom; say download |
| `guard-keep-off-leave-in-cloud` | Left in their cloud, where they are safe | Left online, where they are stored now | "their cloud, where they are safe" is an unchecked promise |
| `guard-safe-in` | Safe in { $provider } | Stored in { $provider } | "Safe" is a promise PCTwin cannot check; state where |
| `guard-safe-in-cloud` | Safe in your cloud | Stored online | "Safe in your cloud": unchecked promise plus jargon |
| `guard-your-cloud` | your cloud | your online storage | "cloud" is jargon for many people |
| `changed-strip` | Changes since you said yes | Changes since you approved | "said yes" is cute; use approve |
| `changed-title` | { $count -> [one] { $count } thing changed after you said yes *[other] { $count } things changed after you said yes } | { $count -> [one] { $count } thing changed after you approved *[other] { $count } things changed after you approved } | "said yes" is cute; use approve |
| `changed-body` | Only these need a fresh yes. Everything else stays approved. | Only these need your approval again. Everything else stays approved. | "a fresh yes" is cute |
| `changed-approve` | Say yes again | Approve again | Button: verb the product uses everywhere else |
| `changed-why-app-updated` | The app was updated after you said yes | The app was updated after you approved | "said yes" is cute; use approve |
| `changed-why-app-removed` | The app was removed after you said yes | The app was removed after you approved | "said yes" is cute; use approve |
| `changed-why-project-edited` | It was changed on the old laptop after you said yes | It was changed on the old laptop after you approved | "said yes" is cute; use approve |
| `repair-waits-administrator` | Needs your yes to the administrator prompt | Needs an administrator’s approval | "your yes to the administrator prompt" is cute and technical |
| `guard-permission-title-denied` | You said no | You did not approve | "You said no" reads as blame; state the fact |
| `guard-permission-body-closed-without-answer` | Closing it is not a yes, so nothing that needs it was done. Ask again, or carry on without it. | Closing the window does not count as approval, so nothing that needs it was done. Ask again, or continue without it. | "not a yes" is cute; say what closing means |
| `guard-permission-not-now` | Carry on without it | Continue without it | Button: "Continue" is the standard verb (Windows, macOS) |

## Progress

Sources and the pattern taken from each:

- **Apple Support, Use Quick Start to transfer data to a new iPhone (support.apple.com/102659)**: Plain, factual lines while it runs: "Keep your devices near each other and plugged in", "Transfer times can vary".
- **Microsoft Support, Transfer your files and settings to a new Windows PC**: Keep both PCs plugged in throughout; status is stated, not dressed up.
- **Microsoft Windows UX guidelines**: Standard verbs on buttons (Continue, Resume, Close), so nobody has to read them twice.

| Key | Old | New | Why |
|---|---|---|---|
| `progress-title` | Settling you in. | Moving your things to this laptop | Founder rejected "Settling you in"; say what is happening |
| `progress-compact` | Keep it compact | Show less | Button: standard show/hide pair |
| `progress-expand` | See the whole move | Show every item | Button: say what appears |
| `progress-all-title` | See the whole move. | Every item in the move | Heading says what the list is |
| `mini-title` | Your move, nearby. | Move in progress | "Your move, nearby." is a tagline |
| `pause-title` | Paused at a safe point. | The move is paused | "safe point" is engine talk; state the fact |
| `pause-title-finishing` | Pausing at a safe point… | Pausing the move… | "safe point" is engine talk; the body says why it waits |
| `pause-resume` | Resume saved move | Resume the move | Button: "saved" adds nothing; Windows says Resume |
| `cancel-keep` | Keep moving | Continue the move | "Keep moving" is ambiguous; say the action |
| `guard-resume` | Carry on | Resume | Button: the standard verb is Resume |
| `guard-state-paused` | Paused at a safe point | Paused | "safe point" is engine talk |
| `guard-state-pause-requested` | Pausing at the next safe point. Not paused yet. | Pausing after the current step. Not paused yet. | "safe point" is engine talk; say what it waits for |
| `guard-state-reconciling` | Checking where things got to | Checking what was finished | Idiom "where things got to" |
| `guard-checking-title` | Checking where things got to | Checking what was finished | Idiom "where things got to" |
| `drive-problem-carry-on` | Carry on | Continue | Button: the standard verb is Continue |
| `drive-problem-fix` | Check that the drive is plugged in and has free space, then carry on. Nothing you chose is left out to get past this. You can also stop the move. | Check that the drive is plugged in and has free space, then continue. Nothing you chose is left out to get past this. You can also stop the move. | Matches the Continue button |
| `fullmove-close-without` | Go on without them | Continue without them | Button: the standard verb is Continue |

## Quick Send

Sources and the pattern taken from each:

- **Google Android Help, Quick Share**: Receive mode, Accept or Decline, and a plain status for each side.
- **WeTransfer Help Center, Download a transfer (wetransfer.com/help-center/how-to/download-a-transfer)**: Buttons say exactly what happens ("Download all"). Its live screen titles could not be confirmed, so none are copied.
- **Microsoft Writing Style Guide, Top 10 tips**: Fewer words: "Sending files" over "Your files are on their way".

| Key | Old | New | Why |
|---|---|---|---|
| `qs-compact` | Keep compact | Show less | Button: standard show/hide pair |
| `mini-title-send` | Your files are on their way. | Sending files | Idiom "on their way"; plain status (Google Drive: "Uploading") |
| `qs-sending-title` | Your files are on their way. | Sending your files | Idiom "on their way"; plain status |
| `qs-receiving-title` | Files are arriving. | Receiving files | Same pattern as the sending title |
| `qs-done-here-title` | Your files are here. | The files were received | "Your files are here" is vague; state the result |

## Results

Sources and the pattern taken from each:

- **Samsung Smart Switch guides**: When the transfer is complete the screen says so and offers Done; no greeting.
- **Apple Support, Migration Assistant**: "After Migration Assistant is done, quit... then log in": the finish states the fact and the next step.
- **Microsoft Windows UX guidelines, Error messages**: Be specific: names, places and numbers, never a general claim the app has not checked.

| Key | Old | New | Why |
|---|---|---|---|
| `outcome-title` | Welcome to your next laptop. | The move is finished | Founder rejected "Welcome to your next laptop"; state the fact, the body gives the result |
| `outcome-title-old` | Your things are on your new laptop. | The move is finished | Claimed success before the report proves it; state the fact |
| `ready-eyebrow` | Welcome | From your old laptop | "Welcome" is greeting-card warmth; say where the things came from |
| `ready-title` | Your things are ready. | Your things are in your folders | "ready" was vague; say where they are |
| `pickup-title` | Pick up safely. | Tidy up the stopped move | "Pick up safely" is an idiom; say the task |
| `pickup-done` | All tidy. Pair the laptops again to carry on: what is already here is found and not copied twice. | Tidying up is finished. Pair the laptops again to continue: what is already here is found and not copied twice. | "All tidy" is chirpy |
| `guard-report-title-partial` | Most things moved. A few did not. | Some things did not move | "Most" may be false; say only what is known |

## Errors

Sources and the pattern taken from each:

- **Microsoft Windows UX guidelines, Error messages**: Problem, cause, solution; do not blame; "unknown error" messages are useless; never OK on an error.
- **GOV.UK Design System, Error message**: Say what went wrong and how to fix it; no "please" when there is only one way forward.
- **Mailchimp Content Style Guide, TL;DR**: Familiar words ("read", "give access") over idioms ("looked in", "let PCTwin in").

| Key | Old | New | Why |
|---|---|---|---|
| `move-failed-title` | Something went wrong. | The move stopped because of an error | "Something went wrong" says nothing (Windows error guidelines); say what happened |
| `fda-title` | Your Mac is keeping some folders private. | PCTwin needs permission to read some folders | Personifies the Mac; say what PCTwin needs |
| `fda-body` | { NUMBER($count) -> [one] { $count } of your places could not be looked in, because of your Mac’s privacy settings. Let PCTwin in, then check again, so nothing of yours is missed. *[other] { $count } of your places could not be looked in, because of your Mac’s privacy settings. Let PCTwin in, then check again, so nothing of yours is missed. } | { NUMBER($count) -> [one] { $count } place could not be read because of your Mac’s privacy settings. Give PCTwin access, then check again, so none of your files are missed. *[other] { $count } places could not be read because of your Mac’s privacy settings. Give PCTwin access, then check again, so none of your files are missed. } | "looked in", "Let PCTwin in" are idioms; say read and give access |
| `fda-without` | Go on without them | Continue without them | Button: the standard verb is Continue |

## Undo and history

Sources and the pattern taken from each:

- **Dropbox Help, Recover deleted files (help.dropbox.com/delete-restore/recover-deleted-files-folders)**: Plain nouns and verbs ("Deleted files", Restore) and an exact statement of the result: it "restores files to the folder they were deleted from".
- **Microsoft Writing Style Guide, Top 10 tips**: Titles are short noun phrases without end punctuation ("Your move history").

| Key | Old | New | Why |
|---|---|---|---|
| `history-title` | Your moves, kept on this laptop. | Your move history | Sentence-as-title; the note below already says it stays on this laptop |
| `history-status-stopped` | Stopped safely | Stopped | "safely" is reassurance the status cannot carry |

## Finish your move

Sources and the pattern taken from each:

- **Apple Support, Migration Assistant**: The closing steps are concrete actions on a named machine.
- **Shopify Polaris, Button**: Labels are actions with a noun ("Copy recent changes"), never a line spoken as the user ("Bring me up to date").
- **Microsoft Writing Style Guide, Top 10 tips**: "Revise weak writing": cut chatty questions such as "Can I actually work?".

| Key | Old | New | Why |
|---|---|---|---|
| `finish-headline-data-preserved` | Your things are kept safe, but some cannot be opened yet | Everything is here, but some of it needs a key to open | "kept safe" is vague; say what is blocking |
| `finish-step-catchup` | Bring me up to date | Copy recent changes | "Bring me up to date" speaks as the user; say the action |
| `finish-catchup-title` | Bring me up to date | Copy recent changes | Same as the step that opens it |
| `work-strip` | Can I work? | Work check | Question-as-label; name the check |
| `work-title` | Can I actually work? | Check that you can work here | "Can I actually work?" is chatty; say what the page does |
| `finish-step-work` | Can I actually work? | Check that you can work here | Same as the page it opens |
| `work-label-still-needs-attention` | Still needs a look | Still needs checking | Idiom "a look" |
| `finish-handover-check-armed` | PCTwin is ready to check the old laptop. Open PCTwin on the old laptop and connect the two laptops as for a move. Then press Look again. | PCTwin is ready to check the old laptop. Open PCTwin on the old laptop and connect the two laptops as for a move. Then choose Look again. | "press" vs the app’s own "choose" |
| `finish-started-body` | The steps below are the system’s, not PCTwin’s. PCTwin does not draw its own copy of them. | The steps below are the system’s, not PCTwin’s. PCTwin does not show its own version of them. | "draw its own copy" is unclear |
| `guard-rescue-title-intro` | Save what matters from a laptop that will not start | Copy files from a laptop that will not start | "Save what matters" is a slogan |
| `guard-rescue-body-pick-drive` | Choose the old laptop’s drive. The drive things go to cannot be the one you rescue. | Choose the old laptop’s drive. The drive you copy to cannot be the one you rescue. | "The drive things go to" was hard to parse |

## Headings: final full stop removed

Source: Microsoft Writing Style Guide, Top 10 tips: "Don't use a period or a colon at the end of titles, headings, subheadings, and UI titles." Done in all seven languages (., 。, ।, ۔). The home headline `welcome-title-line2` keeps its full stop: it is the founder's tagline, "Send files, or move everything."

| Key | Now |
|---|---|
| `name-title` | Name this laptop |
| `home-update-title` | A new version is available |
| `connect-prep-title` | Put both laptops together |
| `reconnect-title` | Waiting for your other laptop |
| `ended-title` | The connection has ended |
| `pair-heads-up-title` | Your laptop may ask you a question |
| `pair-old-locked-title` | Someone on this network tried wrong codes |
| `pair-network-title` | Choose your Wi-Fi |
| `pair-choose-title` | Choose the matching number |
| `pair-new-search-failed-title` | Could not look for your old laptop |
| `pair-new-none-title` | No old laptop found |
| `pair-new-code-title` | Enter the code |
| `pair-new-number-title` | Choose this number on your old laptop |
| `pair-done-title` | Your laptops are paired |
| `people-title` | Match people to their profiles |
| `before-title` | Before you leave this laptop |
| `help-title` | Get help with this task |
| `progress-title-old` | Sending your things to your new laptop |
| `mini-title-paused` | Move paused |
| `move-wait-scanning-title` | Finding your files |
| `move-wait-approval-title` | Waiting for your new laptop |
| `move-wait-list-title` | Waiting for your old laptop |
| `move-wait-starting-title` | Starting the move |
| `move-wait-admin-title` | Waiting for an administrator |
| `move-lost-title` | The connection to your other laptop was lost |
| `move-not-approved-title` | The move did not start |
| `move-refused-title` | The move could not start |
| `drive-problem-title` | The move needs your attention |
| `mini-title-send-paused` | Send paused |
| `outcome-stopped-title` | Your move was stopped |
| `outcome-broke-title` | Your move ended early |
| `qs-problem-lost-title` | The connection between the laptops was lost |
| `qs-problem-failed-title` | The files could not be sent this time |
| `qs-home-title` | Send files to a laptop nearby |
| `qs-pair-new-search-failed-title` | Could not look for the sending laptop |
| `qs-pair-new-none-title` | No sending laptop found |
| `qs-pair-new-number-title` | Choose this number on { $label } |
| `qs-done-sent-title` | Your files were sent |
| `qs-done-partial-title` | Some files did not arrive |
| `qs-done-stopped-title` | This send was stopped |
| `qs-done-empty-title` | Nothing was sent |
| `qs-problem-room-title` | Not enough room in { $folder } |
| `qs-done-refused-title` | The receiving laptop did not take these files |
| `guard-report-title-done` | Everything in this part moved |
| `move-wait-appdata-title` | Looking at your app data |
| `space-box-title` | This laptop does not have room for everything |
