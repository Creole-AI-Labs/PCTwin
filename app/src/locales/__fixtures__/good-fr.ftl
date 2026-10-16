# A correct French translation of source.ftl. The gates must find nothing wrong here.

app-name = PCTwin
greet = Bonjour { $name }, content de vous revoir.
sizes = { NUMBER($value, maximumFractionDigits: 1) } Go libres
back = Retour
photo-button =
    .aria-label = Ouvrir la photo de { $name }
files-found =
    { NUMBER($count) ->
        [0] Aucun fichier trouvé.
        [one] { $count } fichier trouvé.
        [many] { $count } de fichiers trouvés.
       *[other] { $count } fichiers trouvés.
    }
bold-note = Appuyez sur <b>Démarrer</b> pour commencer, ou lisez l’<a>aide</a>.
fee-note = Chats &amp; chiens restent gratuits.
wifi-note = Utilisez le Wi-Fi pour déplacer vos fichiers.
brand-note = PCTwin garde vos fichiers en sécurité.
missing-one = Cette chaîne est traduite ici.
long-note = Choisissez un ordinateur.
label-pick =
    { $gender ->
        [female] Son ordinateur
       *[other] Leur ordinateur
    }
label-colour-green =
    { $gender ->
        [feminine] verte
       *[masculine] vert
    }
label-colour-blue =
    { $gender ->
        [feminine] bleue
       *[masculine] bleu
    }
label-animal-fox = Renard
label-gender-fox = masculine
items-left =
    { $count ->
        [one] { $count } élément restant
        [many] { $count } d’éléments restants
       *[other] { $count } éléments restants
    }
