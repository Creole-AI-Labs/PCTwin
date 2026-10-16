# Planted bad strings. Each id says what is wrong; the meta-test expects that exact gate to fail.

app-name = PCTwin
# placeholder: the variable is renamed
greet = Bonjour { $nom }, content de vous revoir.
# placeholder: function options dropped
sizes = { NUMBER($value) } Go libres
back = Retour
# placeholder: variable lost inside an attribute
photo-button =
    .aria-label = Ouvrir la photo
# plural: the many category is missing
files-found =
    { NUMBER($count) ->
        [0] Aucun fichier trouvé.
        [one] { $count } fichier trouvé.
       *[other] { $count } fichiers trouvés.
    }
# markup: closing tag lost
bold-note = Appuyez sur <b>Démarrer pour commencer, ou lisez l’<a>aide</a>.
# markup: entity changed
fee-note = Chats & chiens restent gratuits.
# never-translate: Wi-Fi translated
wifi-note = Utilisez le réseau sans fil pour déplacer vos fichiers.
# never-translate: brand name altered
brand-note = PCTwln garde vos fichiers en sécurité.
# missing-one is left out on purpose (completeness)
# length: far too long for the English
long-note = Veuillez choisir un ordinateur portable parmi tous ceux qui sont proposés dans la liste ci-dessous.
# plural-default and non-plural select: the default is not [other]/missing
label-pick =
    { $gender ->
       *[female] Son ordinateur
        [other] Leur ordinateur
    }
# extra id that English does not have
not-in-english = Surprise.
# plural: the default variant is [one], not [other]
items-left =
    { $count ->
       *[one] { $count } élément restant
        [many] { $count } d’éléments restants
        [other] { $count } éléments restants
    }
# gender helper: the value is not one of masculine/feminine/neuter
label-gender-fox = male
# gender helper: no such animal in English
label-gender-unicorn = masculine
# gender select without a default variant (a syntax error)
label-colour-green =
    { $gender ->
        [feminine] verte
        [masculine] vert
    }
# gender select with a key that is not a gender
label-colour-blue =
    { $gender ->
        [male] bleu
       *[masculine] bleu
    }
label-animal-fox = Renard
