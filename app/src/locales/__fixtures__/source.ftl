# Small English source for the meta-test. Not used by the app.

app-name = PCTwin
greet = Hello { $name }, welcome back.
sizes = { NUMBER($value, maximumFractionDigits: 1) } GB free
back = Back
photo-button =
    .aria-label = Open the photo from { $name }
files-found =
    { NUMBER($count) ->
        [0] No files found.
        [one] { $count } file found.
       *[other] { $count } files found.
    }
bold-note = Press <b>Start</b> to begin, or read the <a>help</a>.
fee-note = Cats &amp; dogs stay free.
wifi-note = Use Wi-Fi to move your files.
brand-note = PCTwin keeps your files safe.
missing-one = This string is deliberately not translated in the bad file.
long-note = Pick a laptop.
label-pick =
    { $gender ->
        [female] Her laptop
       *[other] Their laptop
    }
items-left =
    { $count ->
        [one] { $count } item left
       *[other] { $count } items left
    }
label-colour-green = Green
label-colour-blue = Blue
label-animal-fox = Fox
