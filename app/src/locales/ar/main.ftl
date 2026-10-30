# PCTwin screens, Arabic (ar). Translated ahead of time from ../en/main.ftl;
# a string missing here falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = صفحة PCTwin الرئيسية
language-label = اللغة
motion-label = الحركة
motion-system = الحركة: مثل النظام
motion-reduced = الحركة: مخففة
motion-off = الحركة: متوقفة

# The opening (Wordplay); its tagline is the home headline.
opening-label = PCTwin قيد الفتح
opening-status = جارٍ فتح PCTwin.
opening-skip = تخطي المقدمة
opening-tagline = أرسل ملفات، أو انقل كل شيء.

footer-free = مجاني. لا حاجة إلى حساب.
footer-help = مساعدة

## F01 Home

welcome-title-line1 = أرسل ملفات،
welcome-title-line2 = أو انقل كل شيء.
welcome-body = أرسل بضعة ملفات إلى حاسوب محمول قريب، أو انقل ملفاتك وتطبيقاتك وإعداداتك إلى حاسوبك المحمول الجديد.
welcome-start = بدء نقل كامل

## F01 Home: the cards, My history, a move to resume, files waiting

home-quick-send-body = أرسل الملفات أو استقبلها
home-move-body = انقل إعداداتك
home-restore-body = استعادة إعداد محفوظ
home-rescue-body = انسخ الملفات من حاسوب محمول لا يبدأ التشغيل
home-history = السجل
home-history-body = عمليات النقل وتقاريرها
name-title = سمِّ هذا الحاسوب
name-body = ستجد الحواسيب القريبة هذا الحاسوب بهذا الاسم. يمكنك تغييره لاحقًا عند الاقتران.
name-use = استخدم هذا الاسم
home-update-title = يتوفر إصدار جديد
home-update-action = حدّث الآن
home-updating = جارٍ التحديث. سيُغلق PCTwin ثم يُفتح من جديد تلقائيًا.
home-update-failed = تعذّر تثبيت التحديث. يعمل PCTwin كما كان؛ وسيُعرض التحديث مرة أخرى في المرة القادمة.
home-update-busy = أكمل عملية النقل أولًا. ثم حدّث PCTwin.
home-resume-body = تم حفظ تقدمك.
home-resume-action = متابعة
    .aria-label = استئناف عملية النقل
home-waiting =
    { NUMBER($count) ->
        [zero] { $count } ملف بانتظار { $to }
        [one] ملف واحد بانتظار { $to }
        [two] ملفان بانتظار { $to }
        [few] { $count } ملفات بانتظار { $to }
        [many] { $count } ملفًا بانتظار { $to }
       *[other] { $count } ملف بانتظار { $to }
    }

## Header: this laptop, the language and motion control, Help

header-this-laptop = حاسوبك

## F06 Old or new laptop

role-back = رجوع
role-eyebrow = بدء نقل كامل
role-title = أي حاسوب محمول هذا؟
role-body = افتح PCTwin على الحاسوبين المحمولين، ثم حدّد أيهما هذا.
role-old-title = هذا حاسوبي المحمول القديم
role-old-body = إرسال ملفاتي وتطبيقاتي وإعداداتي.
role-new-title = هذا حاسوبي المحمول الجديد
role-new-body = استلام إعدادي الكامل من حاسوب محمول آخر.
role-note = لا يُنقل أي شيء قبل أن تراجع الخطة وتوافق عليها. يبقى حاسوبك المحمول القديم كما هو أثناء النقل.

## Frame parts (footer note, Back, steps)

footer-note-move = نقل شخصي مجاني
footer-note-send = إرسال قريب مجاني
frame-back = رجوع
# Read out by screen readers only; the step shapes carry the meaning on screen.
frame-step-done = تم
frame-step-current = الآن
frame-step-todo = التالي
frame-step-position = الخطوة { $number } من { $total }

## F01 Unfinished move

resume-open = استئناف عملية النقل

## C01 How will you connect?

connect-eyebrow = عملية النقل
connect-title = كيف سيتصل الحاسوبان؟
connect-body = مهما كانت الطريقة التي تختارها، فالخطوات نفسها.
connect-wifi-title = Wi-Fi
connect-wifi-body = الحاسوبان المحمولان على شبكة Wi-Fi نفسها.
connect-wifi-status = متاح
connect-later = الاتصال بكابل شبكة أو بكابل الجسر من PCTwin، والنقل عبر محرك أقراص خارجي، كلها مخطط لها في إصدار لاحق.
connect-usb-note = لا يستطيع كابل USB العادي توصيل حاسوبين محمولين لإجراء النقل.
connect-use-wifi = استخدام Wi-Fi
# Added: sizes and speeds as measured, shown the same way on every connection screen.
connect-unit-mbps = ⁦{ NUMBER($value, maximumFractionDigits: 0) } MB/s⁩
connect-unit-gb = ⁦{ NUMBER($value, maximumFractionDigits: 1) } GB⁩

## C02 Put both laptops together

connect-prep-title = ضع الحاسوبين المحمولين بالقرب من بعضهما
connect-prep-body = وصّلهما بشبكة Wi-Fi نفسها، ثم افتح PCTwin على كل منهما.
connect-prep-step-1-title = الانضمام إلى شبكة Wi-Fi نفسها
connect-prep-step-1-body = نسخ الملفات لا يحتاج إلى الإنترنت.
# $network is the Wi-Fi name this laptop is on.
connect-prep-step-1-network = الشبكة: { $network }.
connect-prep-step-2-title = السماح بالعثور على الحواسيب القريبة
connect-prep-step-2-body = إذا طلب Windows أو macOS الإذن، فاختر «السماح».
connect-prep-step-3-title = اختيار دور على كل حاسوب
connect-prep-step-3-body = أحدهما يرسل، والآخر يستلم.
connect-prep-note = قد تمنع شبكات Wi-Fi العامة أو شبكات الضيوف الحواسيب المحمولة من رؤية بعضها.
connect-prep-continue = المتابعة إلى الربط

## C06 Meet your other laptop

peer-eyebrow = عملية النقل
peer-title = تحقق من الحاسوبين المحمولين
peer-body = الحاسوبان المحمولان متصلان وجاهزان.
peer-sending = إرسال
peer-receiving = استلام
peer-old = الحاسوب المحمول القديم
peer-new = الحاسوب المحمول الجديد
peer-version-label = PCTwin
# $version is the PCTwin version on that laptop, for example "1.0.3".
peer-version = { $version }
peer-status-update = يحتاج إلى تحديث
peer-status-unsupported = في الانتظار
peer-ready-note = لا يعرض PCTwin ملفاتك إلا بعد ربط الحاسوبين المحمولين. إذا اختلف الإصدار، فحدّث PCTwin على كليهما.
peer-version-differs = إصدار PCTwin مختلف على هذين الحاسوبين المحمولين. حدّث PCTwin على كليهما للمتابعة.
peer-version-old-body = إصدار مختلف: { $version }.
peer-version-new-body = يجب أن يكون الإصدار نفسه على الحاسوبين المحمولين.
peer-update-title = تحديث PCTwin
peer-update-body = ثبّت الإصدار نفسه على الحاسوبين المحمولين.
# $from and $to are system names, for example "Windows 11" and "macOS". The mockup sentence does not use them.
peer-direction-unsupported = لا يستطيع هذا الإصدار من PCTwin النقل بين هذين النظامين بعد.
peer-continue = التحقق من الاتصال

## C07 Let’s check the connection

check-eyebrow = عملية النقل
check-title = تحقق من الاتصال
check-body = يتحقق PCTwin من السرعة والمساحة والطاقة ليخطط للنقل ويقدّر الوقت.
check-measuring = جارٍ قياس الاتصال…
# The value next to it is already formatted, for example "62 MB/s".
check-speed-label = سرعة الاتصال
check-space-label = متاحة على الحاسوب الجديد
check-link-weak = الاتصال ضعيف، لذا قد يكون النقل بطيئًا. قد يساعد تقريب الحاسوبين المحمولين من موجّه Wi-Fi.
check-power-title = الطاقة
check-power-both = الحاسوبان المحمولان موصولان بالكهرباء.
check-power-battery = يعمل { $label } على البطارية. وصّله بالكهرباء حتى لا ينقطع النقل الطويل.
check-space-title = التخزين
check-space-body = المساحة المتاحة على الحاسوب المحمول الجديد: { $free }. قبل نقل أي شيء، سترى هل تتسع لاختياراتك.
check-internet-title = الإنترنت
check-internet-on = متصل. نسخ الملفات لا يحتاج إلى الإنترنت.
check-internet-off = غير متصل. لا بأس، فنسخ الملفات لا يحتاج إلى الإنترنت.
check-status-ready = جاهز
check-status-plug-in = وصّله بالكهرباء
check-status-connected = متاح
check-status-not-needed = غير مطلوب
# Added: what is said when something could not be read, so it is never shown as fine.
check-status-unknown = غير معروف
check-speed-unknown = تعذّر على PCTwin قياس الاتصال هذه المرة. ستنتقل الملفات مع ذلك؛ ينقص فقط تقدير الوقت.
check-space-unknown = تعذّر على PCTwin قراءة المساحة المتاحة على الحاسوب المحمول الجديد.
check-power-unknown = تعذّر على PCTwin معرفة هل { $label } موصول بالكهرباء.
check-internet-unknown = تعذّر على PCTwin معرفة هل هذا الحاسوب المحمول متصل بالإنترنت. لا بأس، فنسخ الملفات لا يحتاج إلى الإنترنت.
check-again = التحقق مرة أخرى
check-estimate-note = الوقت المقدّر ليس وعدًا ثابتًا.
check-continue = اختيار ما يُنقل
check-continue-new = متابعة
# Added 23 October 2026: the short checklist both laptops show before the move.
check-before-title = قبل أن يبدأ النقل
check-before-lids-title = أبقِ غطاءي الحاسوبين مفتوحين
check-before-lids-body = قد يجعل إغلاق الغطاء الحاسوب المحمول يدخل في وضع السكون، فيتوقف النقل مؤقتًا.
check-before-near-title = ابقَ قريبًا من موجّه Wi-Fi
check-before-near-body = أبقِ الحاسوبين المحمولين قريبين منه أثناء نقل أغراضك.

## C09 Waiting for your other laptop

reconnect-title = في انتظار حاسوبك المحمول الآخر
reconnect-body = كل ما نُسخ وتم التحقق منه حتى الآن محفوظ.
# Added: the title of the saved-progress card (the mockup shows Move paused by connection).
reconnect-card-title = النقل متوقف مؤقتًا بسبب الاتصال
reconnect-progress =
    .aria-label = النقل متوقف مؤقتًا بسبب الاتصال
# $done is an already formatted size, for example "81.6 GB". $total is passed but not shown.
reconnect-saved = تم نسخ { $done } والتحقق منها حتى الآن. والباقي قادم.
# $ago is already formatted, for example "45 seconds ago".
reconnect-last-seen = آخر ظهور: { $ago }.
reconnect-status-offline = غير متصل
reconnect-status-trying = جارٍ إعادة الاتصال
reconnect-status-pending = في الانتظار
reconnect-trying = يحاول PCTwin إعادة الاتصال تلقائيًا… أبقِ الحاسوبين المحمولين مشغّلين وخارج وضع السكون وعلى شبكة Wi-Fi نفسها.
reconnect-expired = مضى وقت طويل لا يسمح بإعادة الاتصال تلقائيًا. اربط الحاسوبين المحمولين مرة أخرى، وسيتحقق PCTwin مما نُسخ بالفعل.
reconnect-unavailable = تعذّر الوصول إلى { $label }. تأكد أنه مشغّل وخارج وضع السكون، وأن PCTwin مفتوح عليه، وأنه على شبكة Wi-Fi نفسها.
reconnect-try-now = إعادة محاولة الاتصال
reconnect-pair-again = الربط مرة أخرى
reconnect-nothing-new = لم يصل أي شيء جديد منذ مدة. يواصل PCTwin المحاولة تلقائيًا، أو يمكنك إيقاف النقل. كل ما نُسخ وتم التحقق منه حتى الآن يبقى محفوظًا.
reconnect-details = إعادة الاتصال بأمان
reconnect-details-body = لا يمكن أن يعيد الاتصال إلا هذان الحاسوبان المحمولان، وفي كل مرة يستخدمان اتصالًا خاصًا جديدًا.

## C10 The connection has ended (dialog)

ended-title = انتهى الاتصال
ended-body = عملية النقل محفوظة. الرمز القديم لم يعد يعمل.
ended-kept-title = محفوظ
ended-kept-body = سجل عملية النقل وكل ما نُسخ وتم التحقق منه.
ended-not-kept-title = غير محفوظ
ended-not-kept-body = الإذن بالمتابعة دون الربط مرة أخرى.
ended-pair-again = الربط مرة أخرى
ended-pair-again-body = أعد الاتصال، وسيتحقق PCTwin مما نُسخ بالفعل.
ended-see-moved = فتح عملية النقل المحفوظة
ended-see-moved-body = اطّلع على ما نُسخ وتم التحقق منه حتى الآن.
ended-home = العودة إلى الرئيسية

## Pairing, shared

pair-back = رجوع
pair-try-again = إعادة المحاولة
pair-more-details = مزيد من التفاصيل
pair-heads-up-title = قد يطرح عليك حاسوبك سؤالًا
pair-heads-up-local-network = سيسألك جهاز Mac هل يمكن لـ PCTwin العثور على الأجهزة في شبكتك المحلية. اختر «السماح» حتى يتمكن الحاسوبان من العثور على بعضهما.
pair-heads-up-firewall = قد يسألك Windows هل يمكن لـ PCTwin قبول الاتصالات. اختر «السماح» حتى يتمكن حاسوبك الجديد من الوصول إلى هذا الحاسوب.
pair-heads-up-continue = متابعة

## C03 Old laptop: show the code

pair-old-eyebrow = الحاسوب المحمول القديم
pair-old-title = وصّل الحاسوبين المحمولين
pair-old-body = على حاسوبك المحمول الجديد، أدخل الرمز أدناه.
pair-old-next = بعد ذلك، ستطابق رقمًا يظهر على الحاسوبين المحمولين.
pair-old-network = أبقِ الحاسوبين المحمولين على شبكة Wi-Fi نفسها.
pair-old-waiting = في انتظار حاسوبك المحمول الجديد…
pair-old-starting = جارٍ تجهيز رمز…
pair-old-label-title = اسم هذا الحاسوب المحمول
pair-old-label-hint = سيجد حاسوبك المحمول الجديد هذا الحاسوب بهذا الاسم.
pair-old-shuffle = اختيار اسم آخر
pair-old-rename = كتابة اسم
pair-old-rename-label = اسم هذا الحاسوب المحمول
pair-old-rename-note = يمكن لأي شخص على شبكة Wi-Fi هذه رؤية هذا الاسم.
pair-old-rename-save = استخدام هذا الاسم
pair-old-rename-cancel = إلغاء
pair-old-code-meta =
    { $seconds ->
        [zero] يتغير كل دقيقة. رمز جديد بعد { $seconds } ثانية.
        [one] يتغير كل دقيقة. رمز جديد بعد ثانية واحدة.
        [two] يتغير كل دقيقة. رمز جديد بعد ثانيتين.
        [few] يتغير كل دقيقة. رمز جديد بعد { $seconds } ثوانٍ.
        [many] يتغير كل دقيقة. رمز جديد بعد { $seconds } ثانية.
       *[other] يتغير كل دقيقة. رمز جديد بعد { $seconds } ثانية.
    }
pair-old-steps-title = على حاسوبك المحمول الجديد
pair-old-step-1 = افتح PCTwin واختر «حاسوبي المحمول الجديد».
pair-old-step-2 = إذا ظهرت قائمة، فاختر اسم هذا الحاسوب المحمول.
pair-old-step-3 = اكتب الرمز المكوّن من ستة أرقام الظاهر هنا.
pair-old-wait-title = انتظر رمزًا جديدًا
pair-old-wait-body =
    { $seconds ->
        [zero] لم تنجح هذه المحاولة. يظهر رمز جديد بعد { $seconds } ثانية.
        [one] لم تنجح هذه المحاولة. يظهر رمز جديد بعد ثانية واحدة.
        [two] لم تنجح هذه المحاولة. يظهر رمز جديد بعد ثانيتين.
        [few] لم تنجح هذه المحاولة. يظهر رمز جديد بعد { $seconds } ثوانٍ.
        [many] لم تنجح هذه المحاولة. يظهر رمز جديد بعد { $seconds } ثانية.
       *[other] لم تنجح هذه المحاولة. يظهر رمز جديد بعد { $seconds } ثانية.
    }
pair-old-locked-title = حاول شخص ما على هذه الشبكة إدخال رموز خاطئة
pair-old-locked-body = بعد خمسة رموز خاطئة، توقف الربط لحماية هذا الحاسوب المحمول. لم تتم مشاركة أي شيء. إذا كنت أنت من حاول، فاختر «البدء من جديد» للحصول على رمز جديد.
pair-old-locked-hotspot = إذا كنت لا تعرف من حاول، فوصّل الحاسوبين المحمولين بنقطة الاتصال في هاتفك بدلًا من ذلك، ثم اختر «البدء من جديد».
pair-old-start-again = البدء من جديد
pair-old-timed-out = لم يُختر أي رقم في الوقت المحدد، لذا هذا رمز جديد.
pair-old-declined = لم تتطابق الأرقام، لذا لم تتم مشاركة أي شيء. هذا رمز جديد.
pair-old-left = أوقف الحاسوب المحمول الجديد الربط قبل اختيار رقم، لذا لم تتم مشاركة أي شيء. هذا رمز جديد.
pair-network-title = اختر شبكة Wi-Fi
pair-network-hint = اختر الشبكة التي يتصل بها حاسوبك المحمول الجديد.

## C05 Old laptop: match the number

pair-choose-eyebrow = تأكيد الاتصال
pair-choose-title = اختر الرقم المطابق
pair-choose-body = انظر إلى حاسوبك المحمول الجديد، واختر الرقم الذي يعرضه.
pair-choose-number = الرقم { $number }
pair-choose-confirm = تأكيد هذين الحاسوبين
pair-choose-renewed = ظهرت أرقام جديدة. اختر الرقم الظاهر على حاسوبك المحمول الجديد.
pair-choose-none = لا يطابق أي منها
pair-choose-note = إذا لم يعرض الحاسوب المحمول الجديد أي رقم، أو لم يطابقه أي من هذه الأرقام، فاختر «لا يطابق أي منها».
pair-choose-safe = لم تتم مشاركة أي شيء بعد.
pair-choose-checking = جارٍ التحقق…

## C04 New laptop: find the old laptop and enter the code

pair-new-eyebrow = الحاسوب المحمول الجديد
pair-new-searching-title = جارٍ البحث عن حاسوبك المحمول القديم…
pair-new-searching-body = أبقِ PCTwin مفتوحًا على حاسوبك المحمول القديم، وعلى شبكة Wi-Fi نفسها التي يتصل بها هذا الحاسوب.
pair-new-search-failed-title = تعذّر البحث عن حاسوبك القديم
pair-new-list-title = أيها حاسوبك المحمول القديم؟
pair-new-list-body = اختر الاسم الذي يظهر على شاشة حاسوبك المحمول القديم.
pair-new-same-label = يحمل حاسوب محمول آخر قريب هذا الاسم. تحقق من شاشة الحاسوب القديم.
pair-new-needs-update = يعمل على هذا الحاسوب إصدار أقدم من PCTwin. حدِّث PCTwin عليه، ثم ابحث مرة أخرى.
pair-new-none-title = لا يوجد حاسوب محمول قديم
pair-new-none-body = تأكد أن PCTwin مفتوح على حاسوبك المحمول القديم ويعرض رمزًا، وأن الحاسوبين على شبكة Wi-Fi نفسها. ثم ابحث مرة أخرى.
pair-new-search-again = البحث مرة أخرى
pair-old-same-role = يوجد حاسوب آخر قريب مضبوط أيضًا على أنه الحاسوب القديم. يجب أن يكون أحدهما الحاسوب الجديد.
pair-old-switch = هذا هو الحاسوب الجديد
pair-new-none-same-role = هل اختار الحاسوب الآخر أيضًا الحاسوب الجديد؟ إذن اختر الحاسوب القديم عليه، أو بدّل هذا الحاسوب.
pair-new-switch = هذا هو الحاسوب القديم
pair-new-code-title = أدخل الرمز
pair-new-code-body = اكتب الرمز الظاهر على { $label }.
pair-new-code-label = رمز من ستة أرقام
pair-new-code-hint = اقرأه من شاشة حاسوبك المحمول القديم. يتغير كل دقيقة.
pair-new-connect = اتصال
pair-new-connecting = جارٍ الاتصال…
pair-new-other-laptop = اختيار حاسوب محمول آخر
pair-new-number-eyebrow = تأكيد الاتصال
pair-new-number-title = اختر هذا الرقم على حاسوبك المحمول القديم
pair-new-number-tile = رقمك
pair-new-number-label = رقمك هو { $number }
pair-new-number-body = يعرض حاسوبك المحمول القديم ثلاثة أرقام. اختر هذا الرقم هناك.
pair-new-number-waiting = في انتظار حاسوبك المحمول القديم…

## Paired (both laptops)

pair-done-eyebrow = تم الربط
pair-done-title = تم ربط الحاسوبين المحمولين
pair-done-body = حاسوباك المحمولان متصلان. بعد ذلك، يبحث حاسوبك القديم عن ملفاتك، وتوافق أنت على ما يُنقل على الحاسوب الجديد. لا يُنقل شيء قبل موافقتك.
pair-done-continue = متابعة

## Pairing problems (each says what happened, then what to do)

pairing-code-invalid = يجب أن يتكون الرمز من ستة أرقام. تحقق من شاشة الحاسوب المحمول القديم واكتبه مرة أخرى.
pairing-code-wrong = الرمز غير مطابق. تحقق من شاشة الحاسوب المحمول القديم وحاول مرة أخرى.
pairing-code-expired = تغيّر الرمز. اكتب الرمز الجديد من شاشة الحاسوب المحمول القديم.
pairing-busy = الحاسوب المحمول القديم مشغول بالربط مع حاسوب آخر. انتظر لحظة، ثم حاول مرة أخرى.
pairing-wait = يجهّز الحاسوب المحمول القديم رمزًا جديدًا. انتظر حتى يظهر، ثم اكتب الرمز الجديد.
pairing-locked = توقف الربط بعد محاولات خاطئة كثيرة. على الحاسوب المحمول القديم، اختر «البدء من جديد».
pairing-unreachable = تعذّر على هذا الحاسوب المحمول الوصول إلى الحاسوب القديم. تأكد أن الحاسوبين على شبكة Wi-Fi نفسها، ثم حاول مرة أخرى.
pairing-wrong-number = هذا الرقم لا يطابق رقم الحاسوب المحمول الجديد. لم تتم مشاركة أي شيء. اكتب الرمز الجديد على الحاسوب الجديد لتحاول مرة أخرى.
pairing-no-match = لم تتطابق الأرقام على الحاسوب المحمول القديم. لم تتم مشاركة أي شيء. اكتب الرمز الجديد من شاشة الحاسوب القديم.
pairing-timed-out = لم يُختر أي رقم في الوقت المحدد. حاول مرة أخرى بالرمز الجديد على الحاسوب المحمول القديم.
pairing-connection-lost = انقطع الاتصال بين الحاسوبين المحمولين. لم تتم مشاركة أي شيء. حاول مرة أخرى.
pairing-connection-lost-new = انقطع الاتصال بالحاسوب المحمول القديم. لم تتم مشاركة أي شيء.
pairing-different-version = إصدار PCTwin مختلف على الحاسوبين المحمولين. حدّث PCTwin على كليهما، ثم حاول مرة أخرى.
pairing-name-invalid = لا يمكن استخدام هذا الاسم. جرّب اسمًا أقصر فيه حروف وأرقام.
pairing-no-network = هذا الحاسوب المحمول غير متصل بشبكة Wi-Fi أو بشبكة سلكية. وصّله بشبكة، ثم حاول مرة أخرى.
pairing-choose-network = هذا الحاسوب المحمول متصل بأكثر من شبكة. اختر الشبكة التي يتصل بها حاسوبك الجديد:
pairing-local-network-refused = ربما يمنع جهاز Mac تطبيق PCTwin من العثور على الحواسيب المحمولة في هذه الشبكة. إذا سألك جهاز Mac للتو، فاختر «السماح» ثم حاول مرة أخرى. وإلا فافتح «إعدادات النظام»، واختر «الخصوصية والأمن» ثم «الشبكة المحلية»، وفعّل PCTwin، ثم حاول مرة أخرى.
pairing-local-network-refused-details = قد تمنع أيضًا شبكة VPN أو جدار حماية أو تطبيق تصفية الحواسيب من العثور على بعضها. إذا كان أحدها يعمل، فأوقفه أثناء الربط، ثم حاول مرة أخرى.
pairing-failed = لم ينجح الربط هذه المرة. لم تتم مشاركة أي شيء. حاول مرة أخرى.
pairing-failed-new = لم ينجح الاقتران هذه المرة. لم تتم مشاركة أي شيء. اكتب الرمز الجديد من شاشة الحاسوب القديم.

## M06 Match people to their profiles (Who’s moving?)

people-eyebrow = عملية النقل
people-title = طابِق كل شخص مع حسابه
people-body = اختر أين تذهب ملفات كل شخص على الحاسوب المحمول الجديد.
people-col-old = الحاسوب المحمول القديم
people-col-new = الحاسوب المحمول الجديد
people-col-status = الحالة
people-you = أنت
# $name is the person's name on the old laptop. $account is an account name on the new laptop.
people-select =
    .aria-label = حساب { $name } على الحاسوب المحمول الجديد
people-option-existing = { $account } / الحاسوب المحمول الجديد
people-option-new = إنشاء حساب جديد
people-option-leave-out = استبعاد
people-option-choose = اختيار…
people-new-account-hint = يطلب الحاسوب المحمول الجديد الإذن قبل أن ينشئ حسابًا.
people-suggested = مقترح لأن الأسماء متطابقة. تأكد أنه الشخص نفسه.
# $account is an account name on the new laptop.
people-suggested-use = استخدام { $account }
people-status-ready = مؤكد
people-status-waiting = بانتظار موافقة صاحب الحساب
people-status-left-out = مستبعد
people-status-choose = يحتاج إلى اختيارك
people-status-changed = عدّلته أنت
people-waiting-body = يجب على { $name } تسجيل الدخول على الحاسوب المحمول القديم والموافقة قبل نقل الملفات. حتى ذلك الحين، تبقى الملفات دون أي تغيير.
people-just-me = لم يُعثر على الحاسوب المحمول القديم إلا على حسابك أنت.
people-changed-note = غيّرت المكان الذي تذهب إليه ملفات أحد الأشخاص. راجعه قبل المتابعة.
people-shared-title = العناصر المشتركة
people-shared-body = المجلدات المشتركة والمستندات العامة التي تخص كل من يستخدم الحاسوب المحمول القديم.
people-shared-select =
    .aria-label = أين تذهب العناصر المشتركة
people-shared-option-shared = المجلدات المشتركة على الحاسوب المحمول الجديد
people-shared-option-person = مع ملفات { $name }
people-shared-option-leave-out = استبعاد
people-note = لا يغيّر PCTwin كلمة مرور أي شخص أبدًا، ولا يتجاوز تسجيل دخول شخص آخر.
people-list-label = الأشخاص على الحاسوب المحمول القديم
people-todo =
    { $count ->
        [0] لكل من يمكن نقل ملفاته مكان يذهب إليه.
        [zero] لكل من يمكن نقل ملفاته مكان يذهب إليه.
        [one] لا يزال شخص واحد يحتاج إلى اختيار.
        [two] لا يزال شخصان يحتاجان إلى اختيار.
        [few] لا يزال { $count } أشخاص يحتاجون إلى اختيار.
        [many] لا يزال { $count } شخصًا يحتاجون إلى اختيار.
       *[other] لا يزال { $count } شخص يحتاجون إلى اختيار.
    }
people-waiting-count =
    { $count ->
        [zero] ينتظر { $count } شخص الموافقة. يمكنك المتابعة، وتبقى ملفاتهم دون تغيير حتى تتم الموافقة.
        [one] ينتظر شخص واحد الموافقة. يمكنك المتابعة، وتبقى الملفات دون تغيير حتى تتم الموافقة.
        [two] ينتظر شخصان الموافقة. يمكنك المتابعة، وتبقى ملفاتهما دون تغيير حتى تتم الموافقة.
        [few] ينتظر { $count } أشخاص الموافقة. يمكنك المتابعة، وتبقى ملفاتهم دون تغيير حتى تتم الموافقة.
        [many] ينتظر { $count } شخصًا الموافقة. يمكنك المتابعة، وتبقى ملفاتهم دون تغيير حتى تتم الموافقة.
       *[other] ينتظر { $count } شخص الموافقة. يمكنك المتابعة، وتبقى ملفاتهم دون تغيير حتى تتم الموافقة.
    }
people-continue = متابعة

## M26 Before you leave this laptop (things to do on the old laptop)

before-eyebrow = عملية النقل
before-title = قبل أن تترك هذا الحاسوب المحمول
before-body = تحقق من هذه الأمور ما دام حاسوبك المحمول القديم معك.
before-licences-title = تراخيص التطبيقات
before-licences-body = بعض التطبيقات تتطلب منك نقل ترخيصها بنفسك.
before-passwords-title = استعادة الحسابات
before-passwords-body = تأكد أنه لا يزال بإمكانك الدخول إلى حساباتك وإلى أي محركات أقراص مقفلة.
before-codes-title = أبقِ رموز تسجيل الدخول في متناولك
before-codes-body = إذا كان أحد حساباتك يرسل إليك رمزًا أو يستخدم تطبيقًا للتأكد من هويتك، فأبقِ ذلك الهاتف أو التطبيق قريبًا منك.
before-online-title = عمل غير محفوظ
before-online-body = احفظ أي مسودات، وتأكد أن الملفات المخزنة على الإنترنت محدّثة.
before-done =
    .aria-label = أنجزتُ هذا: { $item }
before-note = لا تكتب كلمات المرور أو مفاتيح الاسترداد هنا. إنهاء هذه القائمة لا يعني أن مسح الحاسوب المحمول القديم آمن.
before-empty = لا يلزم فعل أي شيء على حاسوبك المحمول القديم قبل النقل.
before-continue = العودة إلى خطة النقل

## S11 Get help with this task (a short problem code)

help-eyebrow = المساعدة والخصوصية
help-title = احصل على مساعدة في هذه المهمة
help-body = شارك رمزًا قصيرًا يوضح لشخص آخر ما الذي حدث. راجع ما يتضمنه أولًا.
help-making = جارٍ إنشاء رمز…
help-code-title = رمز المشكلة
# $code is the code itself, for example "PCT-7F3K-21". Keep it unchanged.
help-code =
    .aria-label = رمز المشكلة { $code }
help-contains-title = ما يتضمنه
help-contains-problem = نوع المشكلة والمدى الذي وصلت إليه المهمة.
help-contains-step = خطوة النقل التي حدثت فيها المشكلة.
help-contains-versions = إصدار PCTwin، والنظام الذي يعمل عليه هذا الحاسوب، مثل Windows أو macOS.
help-never-title = ما لا يتضمنه أبدًا
help-chip-included = مضمّن
help-chip-excluded = غير مضمّن أبدًا
help-never-names = اسمك أو اسم أي شخص آخر.
help-never-files = محتوى ملفاتك، أو أي كلمات مرور.
help-never-network = أسماء الملفات أو المجلدات، أو مكان حفظ ملفاتك، أو أسماء شبكات Wi-Fi.
help-note = لا يمنح الرمز أي شخص إمكانية الوصول إلى الحاسوبين المحمولين.
help-copy = نسخ الرمز
help-copied = تم نسخ الرمز.
help-share = مشاركة
help-share-hint = أرسله عبر WhatsApp أو أي تطبيق.
help-share-text = رمز مشكلة PCTwin الخاص بي هو { $code }.
help-failed = تعذّر إنشاء رمز المشكلة. حاول مرة أخرى. إذا استمر الأمر، فصِف المشكلة بكلماتك.
help-try-again = إعادة المحاولة
help-copy-failed = تعذّر نسخ الرمز. حدّده في الأعلى وانسخه بنفسك.
help-home-eyebrow = مساعدة
help-home-title = مشكلات شائعة وما يجب فعله
help-home-body = أدلة قصيرة لأكثر ما يعيق عادةً. تعمل دون إنترنت.
help-home-more = مزيد من المساعدة على getpctwin.com
help-guide-find-title = لا يجد الحاسوبان بعضهما
help-guide-find-body = صِل الحاسوبين بشبكة Wi-Fi نفسها، وليس بشبكة الضيوف. أوقف أي VPN أثناء الاقتران. إن لم ينجح ذلك، شغّل نقطة الاتصال في هاتفك وصِل الحاسوبين بها.
help-guide-mac-network-title = يسأل جهاز Mac عن الشبكة المحلية
help-guide-mac-network-body = اختر السماح حتى يتمكن PCTwin من إيجاد حاسوبك الآخر. إذا اخترت عدم السماح، افتح إعدادات النظام ثم الخصوصية والأمن ثم الشبكة المحلية، وشغّل PCTwin.
help-guide-firewall-title = يسأل Windows عن جدار الحماية
help-guide-firewall-body = اسمح لـ PCTwin على الشبكات الخاصة، مثل Wi-Fi المنزل. يتواصل PCTwin مع حاسوبك الآخر فقط، على شبكتك الخاصة.
help-guide-stopped-title = توقف النقل في منتصفه
help-guide-stopped-body = لا يضيع شيء مما نُسخ وتم التحقق منه، ولم يتغير حاسوبك القديم. اقرن الحاسوبين من جديد: تُكتشف الملفات الموجودة على الحاسوب الجديد ولا تُنسخ مرتين.
help-guide-others-title = نقل أشياء أشخاص آخرين
help-guide-others-body = اخترهم في «من سيُنقل؟» على الحاسوب القديم. يوافق المسؤول مرة واحدة على كل حاسوب. لن ترى ملفاتهم أبدًا. تظهر كلمة المرور المؤقتة للحساب الجديد مرة واحدة في نافذة الحاسوب الخاصة، ويختارون كلمتهم عند أول تسجيل دخول.
help-guide-code-title = طلب المساعدة من شخص ما
help-guide-code-body = في أي صفحة مشكلة، اختر «الحصول على مساعدة». ينشئ PCTwin رمزًا قصيرًا يوضح ما حدث، دون أسماء أو ملفات أو كلمات مرور، لترسله عبر WhatsApp أو أي تطبيق.

## Label words (the name each laptop shows while pairing)

label-colour-red =
    { $gender ->
        [feminine] حمراء
       *[masculine] أحمر
    }
label-colour-orange =
    { $gender ->
        [feminine] برتقالية
       *[masculine] برتقالي
    }
label-colour-yellow =
    { $gender ->
        [feminine] صفراء
       *[masculine] أصفر
    }
label-colour-green =
    { $gender ->
        [feminine] خضراء
       *[masculine] أخضر
    }
label-colour-blue =
    { $gender ->
        [feminine] زرقاء
       *[masculine] أزرق
    }
label-colour-purple =
    { $gender ->
        [feminine] بنفسجية
       *[masculine] بنفسجي
    }
label-colour-pink =
    { $gender ->
        [feminine] وردية
       *[masculine] وردي
    }
label-colour-brown =
    { $gender ->
        [feminine] بنية
       *[masculine] بني
    }
label-colour-black =
    { $gender ->
        [feminine] سوداء
       *[masculine] أسود
    }
label-colour-white =
    { $gender ->
        [feminine] بيضاء
       *[masculine] أبيض
    }
label-colour-grey =
    { $gender ->
        [feminine] رمادية
       *[masculine] رمادي
    }
label-colour-gold =
    { $gender ->
        [feminine] ذهبية
       *[masculine] ذهبي
    }

label-animal-ant = نملة
label-animal-badger = غرير
label-animal-bat = خفاش
label-animal-bear = دب
label-animal-bee = نحلة
label-animal-beetle = خنفساء
label-animal-bison = بيسون
label-animal-butterfly = فراشة
label-animal-camel = جمل
label-animal-cat = قطة
label-animal-cheetah = شيتا
label-animal-chicken = دجاجة
label-animal-crab = سلطعون
label-animal-crane = كركي
label-animal-crocodile = تمساح
label-animal-crow = غراب
label-animal-deer = أيل
label-animal-dolphin = دلفين
label-animal-donkey = حمار
label-animal-dove = حمامة
label-animal-dragonfly = يعسوب
label-animal-duck = بطة
label-animal-eagle = نسر
label-animal-elephant = فيل
label-animal-falcon = صقر
label-animal-flamingo = فلامنغو
label-animal-fox = ثعلب
label-animal-frog = ضفدع
label-animal-gazelle = غزال
label-animal-swordfish = أبو سيف
label-animal-giraffe = زرافة
label-animal-goat = عنزة
label-animal-goose = إوزة
label-animal-gorilla = غوريلا
label-animal-hamster = همستر
label-animal-hawk = باز
label-animal-hedgehog = قنفذ
label-animal-armadillo = أرماديلو
label-animal-hippo = فرس النهر
label-animal-horse = حصان
label-animal-hummingbird = طائر طنان
label-animal-jaguar = يغوار
label-animal-jellyfish = قنديل البحر
label-animal-kangaroo = كنغر
label-animal-kingfisher = رفراف
label-animal-koala = كوالا
label-animal-ladybird = دعسوقة
label-animal-lemur = ليمور
label-animal-leopard = فهد
label-animal-lion = أسد
label-animal-lizard = سحلية
label-animal-llama = لاما
label-animal-lobster = كركند
label-animal-lynx = وشق
label-animal-macaw = ماكاو
label-animal-meerkat = ميركات
label-animal-mole = خلد
label-animal-moose = موظ
label-animal-pufferfish = سمكة منتفخة
label-animal-mouse = فأر
label-animal-octopus = أخطبوط
label-animal-ostrich = نعامة
label-animal-otter = قضاعة
label-animal-owl = بومة
label-animal-panda = باندا
label-animal-kiwi = كيوي
label-animal-parrot = ببغاء
label-animal-peacock = طاووس
label-animal-pelican = بجع
label-animal-penguin = بطريق
label-animal-puffin = ببغاء البحر
label-animal-rabbit = أرنب
label-animal-raccoon = راكون
label-animal-rhino = وحيد القرن
label-animal-robin = أبو الحناء
label-animal-salmon = سلمون
label-animal-seahorse = حصان البحر
label-animal-seal = فقمة
label-animal-shark = قرش
label-animal-sheep = خروف
label-animal-sloth = كسلان
label-animal-snail = حلزون
label-animal-sparrow = عصفور
label-animal-squirrel = سنجاب
label-animal-starfish = نجمة البحر
label-animal-stork = لقلق
label-animal-swan = بجعة
label-animal-tiger = نمر
label-animal-snake = ثعبان
label-animal-toucan = توكان
label-animal-turtle = سلحفاة
label-animal-walrus = حيوان الفظ
label-animal-whale = حوت
label-animal-wolf = ذئب
label-animal-woodpecker = نقار الخشب
label-animal-yak = ياك
label-animal-zebra = حمار وحشي
label-animal-dog = كلب
label-animal-cow = بقرة
label-animal-beaver = قندس

label-gender-ant = feminine
label-gender-badger = masculine
label-gender-bat = masculine
label-gender-bear = masculine
label-gender-bee = feminine
label-gender-beetle = feminine
label-gender-bison = masculine
label-gender-butterfly = feminine
label-gender-camel = masculine
label-gender-cat = feminine
label-gender-cheetah = masculine
label-gender-chicken = feminine
label-gender-crab = masculine
label-gender-crane = masculine
label-gender-crocodile = masculine
label-gender-crow = masculine
label-gender-deer = masculine
label-gender-dolphin = masculine
label-gender-donkey = masculine
label-gender-dove = feminine
label-gender-dragonfly = masculine
label-gender-duck = feminine
label-gender-eagle = masculine
label-gender-elephant = masculine
label-gender-falcon = masculine
label-gender-flamingo = masculine
label-gender-fox = masculine
label-gender-frog = masculine
label-gender-gazelle = masculine
label-gender-swordfish = masculine
label-gender-giraffe = feminine
label-gender-goat = feminine
label-gender-goose = feminine
label-gender-gorilla = masculine
label-gender-hamster = masculine
label-gender-hawk = masculine
label-gender-hedgehog = masculine
label-gender-armadillo = masculine
label-gender-hippo = masculine
label-gender-horse = masculine
label-gender-hummingbird = masculine
label-gender-jaguar = masculine
label-gender-jellyfish = masculine
label-gender-kangaroo = masculine
label-gender-kingfisher = masculine
label-gender-koala = masculine
label-gender-ladybird = feminine
label-gender-lemur = masculine
label-gender-leopard = masculine
label-gender-lion = masculine
label-gender-lizard = feminine
label-gender-llama = masculine
label-gender-lobster = masculine
label-gender-lynx = masculine
label-gender-macaw = masculine
label-gender-meerkat = masculine
label-gender-mole = masculine
label-gender-moose = masculine
label-gender-pufferfish = feminine
label-gender-mouse = masculine
label-gender-octopus = masculine
label-gender-ostrich = feminine
label-gender-otter = feminine
label-gender-owl = feminine
label-gender-panda = masculine
label-gender-kiwi = masculine
label-gender-parrot = masculine
label-gender-peacock = masculine
label-gender-pelican = masculine
label-gender-penguin = masculine
label-gender-puffin = masculine
label-gender-rabbit = masculine
label-gender-raccoon = masculine
label-gender-rhino = masculine
label-gender-robin = masculine
label-gender-salmon = masculine
label-gender-seahorse = masculine
label-gender-seal = feminine
label-gender-shark = masculine
label-gender-sheep = masculine
label-gender-sloth = masculine
label-gender-snail = masculine
label-gender-sparrow = masculine
label-gender-squirrel = masculine
label-gender-starfish = feminine
label-gender-stork = masculine
label-gender-swan = feminine
label-gender-tiger = masculine
label-gender-snake = masculine
label-gender-toucan = masculine
label-gender-turtle = feminine
label-gender-walrus = masculine
label-gender-whale = masculine
label-gender-wolf = masculine
label-gender-woodpecker = masculine
label-gender-yak = masculine
label-gender-zebra = masculine
label-gender-dog = masculine
label-gender-cow = feminine
label-gender-beaver = masculine

# Translators: the laptop's name is a colour plus an animal, for example "Blue Fox".
# Arabic: animal first, then the colour, which agrees with the animal's gender.
label-picked = { $animal } { $colour }
label-unnamed = حاسوب محمول بلا اسم

## M02 Finding your files

scan-eyebrow = عملية النقل
scan-title = جارٍ البحث عن ملفاتك
scan-body = يُحفظ ما يجده PCTwin، فيمكن أن يستكمل البحث إذا توقف.
scan-progress =
    .aria-label = جارٍ البحث في الأماكن التي اخترتها
# $size is passed but not shown in the mockup line.
scan-found =
    { NUMBER($count) ->
        [zero] عُثر على { $count } ملف حتى الآن.
        [one] عُثر على ملف واحد حتى الآن.
        [two] عُثر على ملفين حتى الآن.
        [few] عُثر على { $count } ملفات حتى الآن.
        [many] عُثر على { $count } ملفًا حتى الآن.
       *[other] عُثر على { $count } ملف حتى الآن.
    }
scan-step-usual-title = الملفات والمجلدات
scan-step-usual-body = تم البحث في الأماكن التي يستطيع PCTwin قراءتها.
scan-step-other-title = مجلدات ومحركات أقراص أخرى
scan-step-other-body = مجلدات ومحركات أقراص تستخدمها خارج الأماكن المعتادة.
scan-step-protected-title = أماكن محمية
scan-step-protected-body =
    { NUMBER($count) ->
        [zero] لا يزال { $count } مكان بحاجة إلى إذنك.
        [one] لا يزال مكان واحد بحاجة إلى إذنك.
        [two] لا يزال مكانان بحاجة إلى إذنك.
        [few] لا تزال { $count } أماكن بحاجة إلى إذنك.
        [many] لا يزال { $count } مكانًا بحاجة إلى إذنك.
       *[other] لا يزال { $count } مكان بحاجة إلى إذنك.
    }
scan-step-done = تم
scan-step-current = الآن
scan-step-todo = التالي
scan-unreadable-note = الأماكن التي تعذّر على PCTwin قراءتها تبقى في القائمة، ولا تُحسب ضمن ما تم البحث فيه.
scan-status-not-searched = لم يتم البحث فيه
scan-paused = البحث متوقف مؤقتًا. كل ما عُثر عليه حتى الآن محفوظ.
scan-resumed = استُؤنف البحث من حيث توقف.
scan-partial =
    { NUMBER($count) ->
        [zero] لم يتم البحث بعد في { $count } مكان محمي.
        [one] لم يتم البحث بعد في مكان محمي واحد.
        [two] لم يتم البحث بعد في مكانين محميين.
        [few] لم يتم البحث بعد في { $count } أماكن محمية.
        [many] لم يتم البحث بعد في { $count } مكانًا محميًا.
       *[other] لم يتم البحث بعد في { $count } مكان محمي.
    }
scan-allow = السماح بالوصول
scan-pause = إيقاف مؤقت
scan-resume = متابعة البحث
scan-continue = عرض ما عُثر عليه

## M03 Everything you chose, in view

overview-eyebrow = عملية النقل
overview-title = ما وجده PCTwin
overview-body = راجع ما عثر عليه PCTwin قبل أن يُعدّ الخطة.
# The metric value next to each label is passed in already formatted.
overview-size-label = الملفات المحددة
overview-items-label =
    { NUMBER($count) ->
        [zero] ملف
        [one] ملف
        [two] ملف
        [few] ملفات
        [many] ملفًا
       *[other] ملف
    }
overview-attention-label =
    { NUMBER($count) ->
        [zero] بحاجة إلى انتباهك
        [one] بحاجة إلى انتباهك
        [two] بحاجة إلى انتباهك
        [few] بحاجة إلى انتباهك
        [many] بحاجة إلى انتباهك
       *[other] بحاجة إلى انتباهك
    }
overview-files-title = الملفات والمجلدات
overview-files-body = المستندات والصور والمجلدات الأخرى التي اخترتها.
overview-dest-title = أين تذهب
overview-dest-body = المكان المخصص لكل مجلد على الحاسوب المحمول الجديد.
overview-people-title = الأشخاص
overview-people-body = اختر الحساب على الحاسوب المحمول الجديد لكل شخص.
overview-review = مراجعة
overview-partial =
    { NUMBER($count) ->
        [zero] لم يتم البحث بعد في { $count } مكان محمي.
        [one] لم يتم البحث بعد في مكان محمي واحد.
        [two] لم يتم البحث بعد في مكانين محميين.
        [few] لم يتم البحث بعد في { $count } أماكن محمية.
        [many] لم يتم البحث بعد في { $count } مكانًا محميًا.
       *[other] لم يتم البحث بعد في { $count } مكان محمي.
    }
overview-allow = السماح بالوصول
overview-continue = متابعة

## M04 Keep what matters to you

files-eyebrow = عملية النقل
files-title = اختر المجلدات التي تريد نقلها
files-body = اختر المجلدات ومحركات الأقراص الأخرى وأي أماكن أخرى.
files-search-label = البحث عن مجلد
files-search-placeholder = البحث في المجلدات
files-select =
    .aria-label = نقل { $name }
files-folder-detail =
    { NUMBER($count) ->
        [zero] { $size } · { $count } ملف
        [one] { $size } · ملف واحد
        [two] { $size } · ملفان
        [few] { $size } · { $count } ملفات
        [many] { $size } · { $count } ملفًا
       *[other] { $size } · { $count } ملف
    }
files-status-protected = يحتاج إلى إذن
files-protected-body = يحتاج PCTwin إلى إذنك لقراءة هذا المجلد. لا تُحسب ملفاته حتى تسمح بذلك.
files-allow = السماح بالوصول
files-status-added = أضفته أنت
files-add = إضافة مجلد آخر
files-remove =
    .aria-label = إزالة { $name }
files-no-limit = النقل المجاني بلا حد لحجم الملفات أو لعدد المجلدات التي تنقلها.
files-total =
    { NUMBER($count) ->
        [zero] { $count } مجلد مختار، والإجمالي { $size }
        [one] مجلد واحد مختار، والإجمالي { $size }
        [two] مجلدان مختاران، والإجمالي { $size }
        [few] { $count } مجلدات مختارة، والإجمالي { $size }
        [many] { $count } مجلدًا مختارًا، والإجمالي { $size }
       *[other] { $count } مجلد مختار، والإجمالي { $size }
    }
files-none = اختر مجلدًا واحدًا على الأقل للمتابعة.
files-continue = اختيار الوجهات

## M05 Give every file a home

dest-eyebrow = عملية النقل
dest-title = أين تذهب مجلداتك
dest-body = اختر أين يذهب كل مجلد على الحاسوب المحمول الجديد.
dest-col-chosen = ما اخترته
dest-col-place = أين يذهب
dest-col-placement = محرك الأقراص
dest-placement-local = داخلي
dest-placement-external = خارجي
dest-col-status = الحالة
dest-select =
    .aria-label = وجهة { $name }
# $folder is a folder name on the new laptop, for example "Documents".
dest-option-folder = الحاسوب المحمول الجديد / { $folder }
# $drive is the name of a drive connected to the new laptop.
dest-option-drive = محرك أقراص خارجي / { $drive }
dest-option-choose = اختيار مجلد
dest-default-label = المكان المعتاد
dest-status-ready = جاهز
dest-status-missing = محرك الأقراص غير متصل
dest-status-space = المساحة غير كافية
dest-all-ready = لكل مجلد مكان على الحاسوب المحمول الجديد.
dest-missing = محرك الأقراص { $drive } غير موصول بالحاسوب المحمول الجديد. وصّله، أو اختر مكانًا آخر لهذه الملفات.
dest-space = في { $place } مساحة متاحة قدرها { $free }، لكن هذه الملفات تحتاج إلى { $needed }. وفّر مساحة هناك، أو اختر مكانًا آخر، أو انقل ملفات أقل.
dest-change = تغيير
dest-continue = مراجعة الخطة

## M23 Approve the move you want

plan-eyebrow = عملية النقل
plan-title = راجع الخطة ووافق عليها
plan-body = لا يفعل PCTwin إلا ما تنص عليه هذه الخطة.
plan-what-title = ما اخترته
# $size is already formatted, for example "120 GB". $count is passed but not shown.
plan-what-body = ملفات بحجم { $size }، من المجلدات التي اخترتها، إلى الأماكن التي حددتها.
plan-where-body =
    { NUMBER($count) ->
        [zero] إلى { $count } مكان على الحاسوب المحمول الجديد.
        [one] إلى مكان واحد على الحاسوب المحمول الجديد.
        [two] إلى مكانين على الحاسوب المحمول الجديد.
        [few] إلى { $count } أماكن على الحاسوب المحمول الجديد.
        [many] إلى { $count } مكانًا على الحاسوب المحمول الجديد.
       *[other] إلى { $count } مكان على الحاسوب المحمول الجديد.
    }
plan-hand-title = قبل أن تبدأ
plan-hand-body =
    { NUMBER($count) ->
        [zero] { $count } خطوة. ستجدها في تقريرك في النهاية.
        [one] خطوة واحدة. ستجدها في تقريرك في النهاية.
        [two] خطوتان. ستجدهما في تقريرك في النهاية.
        [few] { $count } خطوات. ستجدها في تقريرك في النهاية.
        [many] { $count } خطوة. ستجدها في تقريرك في النهاية.
       *[other] { $count } خطوة. ستجدها في تقريرك في النهاية.
    }
plan-hand-none = لا شيء، بحسب ما يستطيع PCTwin معرفته الآن.
plan-old-safe = يبقى حاسوبك المحمول القديم كما هو. تُنسخ الملفات ولا تُزال منه أبدًا.
plan-edit = تغيير الخطة
plan-edit-note = إذا غيّرت الخطة، فستوافق عليها مرة أخرى قبل أن يبدأ أي شيء جديد.
plan-approve = أوافق على هذه الخطة، بما في ذلك ما يجب أن أفعله بنفسي.
plan-not-approved = لم توافق على هذه الخطة بعد، لذا لن يُنقل أي شيء.
plan-changed = غيّرت الخطة، لذا وافق عليها مرة أخرى قبل نقل أي شيء جديد.
plan-start = بدء النقل

## Plan screens: shared words and the lines the copy deck did not cover

plan-back = رجوع
# $n is a count of files or folders.
plan-number = { NUMBER($n) }
# $n is a number already rounded for reading, for example 18.4.
plan-size-b = { NUMBER($n) } بايت
plan-size-kb = ⁦{ NUMBER($n) } KB⁩
plan-size-mb = ⁦{ NUMBER($n) } MB⁩
plan-size-gb = ⁦{ NUMBER($n) } GB⁩
plan-size-tb = ⁦{ NUMBER($n) } TB⁩
# $size is a size already written out, for example "18.4 GB". Used when part of a folder could not be read.
plan-size-at-least = { $size } على الأقل
plan-size-unknown = الحجم غير معروف
plan-role-home = المجلد الرئيسي
plan-role-desktop = سطح المكتب
plan-role-documents = المستندات
plan-role-downloads = التنزيلات
plan-role-pictures = الصور
plan-role-music = الموسيقى
plan-role-videos = مقاطع الفيديو
plan-role-public = المجلد العام
plan-role-app-data = بيانات التطبيقات
plan-role-other = مجلد آخر
# $done and $total are counts of places this search has looked at and has to look at.
scan-places = الأماكن التي تم البحث فيها: { $done } من { $total }
scan-card-title = جارٍ البحث في الأماكن التي اخترتها
scan-pausing = سيتوقف البحث مؤقتًا بعد المجلد الحالي. كل ما عُثر عليه حتى الآن محفوظ.
overview-row-attention = يحتاج إلى انتباه
files-status-work = العمل أو الدراسة
files-work-body = تدير جهة عملك أو دراستك هذا المجلد، لذا يُستبعد ما لم تحدده.
# $count is how many files in the folder are only online.
files-online-only =
    { NUMBER($count) ->
        [zero] لن يُنسخ { $count } ملف لأنها موجودة على الإنترنت فقط.
        [one] لن يُنسخ ملف واحد لأنه موجود على الإنترنت فقط.
        [two] لن يُنسخ ملفان لأنهما موجودان على الإنترنت فقط.
        [few] لن تُنسخ { $count } ملفات لأنها موجودة على الإنترنت فقط.
        [many] لن يُنسخ { $count } ملفًا لأنها موجودة على الإنترنت فقط.
       *[other] لن يُنسخ { $count } ملف لأنها موجودة على الإنترنت فقط.
    }
files-no-match = لا يوجد مجلد بهذا الاسم.
dest-status-none = اختر مكانًا
dest-status-readonly = لا يمكن الكتابة عليه
dest-status-format = الملف كبير جدًا
# $name is the folder that needs a place.
dest-none = اختر مكانًا للمجلد { $name } على الحاسوب المحمول الجديد.
# $place is a drive on the new laptop that cannot be written to.
dest-readonly = المكان { $place } للقراءة فقط، لذا لا يمكن نسخ أي شيء إليه. اختر مكانًا آخر.
# $place is the drive. Some drive formats cannot hold a single very large file.
dest-format = أحد الملفات في هذه المجلدات كبير جدًا بالنسبة إلى طريقة إعداد { $place }. اختر مكانًا آخر.
# $folder is a folder on the new laptop and $drive the drive it is on.
plan-place = { $folder } على { $drive }
plan-group-to = يذهب إلى { $place }
plan-group-move = نقل { $folder }
plan-place-change = اختر مكانًا آخر
plan-place-usual = استخدم المكان المعتاد
plan-place-free = { $size } متاحة هناك
plan-place-no-room = لا توجد مساحة كافية هناك. اختر مكانًا آخر، أو ألغِ تحديد هذا المجلد.
plan-place-no-safe-name = لا يستطيع PCTwin حفظ الملفات بأمان على ذلك القرص. اختر مكانًا آخر.
plan-place-cannot-open = تعذّر على PCTwin فتح ذلك المجلد. اختر مكانًا آخر.
plan-place-locked = هذا القرص مقفل. افتح قفله أولًا، ثم اختره مرة أخرى.
plan-version = إصدار الخطة { $revision }
plan-details = مزيد من التفاصيل
plan-preview-title = قبل أن تبدأ
plan-preview-fits = توجد مساحة فارغة كافية على هذا الحاسوب.
plan-preview-fits-about = يُفترض أن توجد مساحة فارغة كافية على هذا الحاسوب. يشمل ذلك نحو { $extra } للتطبيقات التي يثبّتها PCTwin وبياناتها.
# Added 23 October 2026: what moves first, from the move's own order. $places is a list of folder names.
plan-preview-first = تُنقل هذه أولًا، لتتمكن من استخدام حاسوبك المحمول الجديد في وقت أبكر: { $places }.
plan-preview-short = يحتاج هذا الحاسوب إلى { $size } إضافية من المساحة الفارغة. وفّر بعض المساحة قبل البدء، وإلا سيتوقف النقل عند امتلاء القرص.
plan-preview-short-about = يحتاج هذا الحاسوب إلى نحو { $size } إضافية من المساحة الفارغة، مع احتساب نحو { $extra } للتطبيقات التي يثبّتها PCTwin وبياناتها. وفّر بعض المساحة قبل البدء، وإلا سيتوقف النقل عند امتلاء القرص.
plan-preview-space-unknown = تعذّر على PCTwin معرفة المساحة الفارغة على هذا الحاسوب.
plan-preview-synced = على هذا الحاسوب، يُحفظ { $folder } في { $service }. الملفات المنقولة إلى هناك سيتم رفعها إلى { $service } وستستخدم مساحتك فيه.
plan-preview-time-one =
    { NUMBER($minutes) ->
        [zero] نحو { $minutes } دقيقة.
        [one] نحو دقيقة واحدة.
        [two] نحو دقيقتين.
        [few] نحو { $minutes } دقائق.
        [many] نحو { $minutes } دقيقة.
       *[other] نحو { $minutes } دقيقة.
    }
plan-preview-time = نحو { $low } إلى { $high } دقيقة.
plan-preview-renamed =
    { NUMBER($count) ->
        [zero] { $count } ملف سيحصل على اسم مختلف قليلًا لأن هذا الحاسوب لا يسمح باسمه. يذكره التقرير.
        [one] ملف واحد سيحصل على اسم مختلف قليلًا لأن هذا الحاسوب لا يسمح باسمه. يذكره التقرير.
        [two] ملفان سيحصلان على اسمين مختلفين قليلًا لأن هذا الحاسوب لا يسمح باسميهما. يذكرهما التقرير.
        [few] { $count } ملفات ستحصل على أسماء مختلفة قليلًا لأن هذا الحاسوب لا يسمح بأسمائها. يذكر التقرير كل ملف.
        [many] { $count } ملفًا سيحصل على اسم مختلف قليلًا لأن هذا الحاسوب لا يسمح بأسمائها. يذكر التقرير كل ملف.
       *[other] { $count } ملف سيحصل على اسم مختلف قليلًا لأن هذا الحاسوب لا يسمح بأسمائها. يذكر التقرير كل ملف.
    }
plan-preview-too-big =
    { NUMBER($count) ->
        [zero] { $count } ملف أكبر من أن يتسع له قرص هذا الحاسوب ولن يُنسخ.
        [one] ملف واحد أكبر من أن يتسع له قرص هذا الحاسوب ولن يُنسخ.
        [two] ملفان أكبر من أن يتسع لهما قرص هذا الحاسوب ولن يُنسخا.
        [few] { $count } ملفات أكبر من أن يتسع لها قرص هذا الحاسوب ولن تُنسخ.
        [many] { $count } ملفًا أكبر من أن يتسع لها قرص هذا الحاسوب ولن تُنسخ.
       *[other] { $count } ملف أكبر من أن يتسع لها قرص هذا الحاسوب ولن تُنسخ.
    }
plan-preview-refused =
    { NUMBER($count) ->
        [zero] { $count } ملف له اسم أو مجلد لا يسمح به هذا الحاسوب، ولن يُنسخ.
        [one] ملف واحد له اسم أو مجلد لا يسمح به هذا الحاسوب، ولن يُنسخ.
        [two] ملفان لهما اسمان أو مجلدان لا يسمح بهما هذا الحاسوب، ولن يُنسخا.
        [few] { $count } ملفات لها أسماء أو مجلدات لا يسمح بها هذا الحاسوب، ولن تُنسخ.
        [many] { $count } ملفًا لها أسماء أو مجلدات لا يسمح بها هذا الحاسوب، ولن تُنسخ.
       *[other] { $count } ملف لها أسماء أو مجلدات لا يسمح بها هذا الحاسوب، ولن تُنسخ.
    }
plan-preview-runs-at-sign-in =
    { NUMBER($count) ->
        [zero] { $count } ملف سيعمل تلقائيًا عند تسجيل الدخول إلى هذا الحاسوب. تأكد من أنك تتوقع ذلك.
        [one] ملف واحد سيعمل تلقائيًا عند تسجيل الدخول إلى هذا الحاسوب. تأكد من أنك تتوقع ذلك.
        [two] ملفان سيعملان تلقائيًا عند تسجيل الدخول إلى هذا الحاسوب. تأكد من أنك تتوقع ذلك.
        [few] { $count } ملفات ستعمل تلقائيًا عند تسجيل الدخول إلى هذا الحاسوب. تأكد من أنك تتوقع ذلك.
        [many] { $count } ملفًا سيعمل تلقائيًا عند تسجيل الدخول إلى هذا الحاسوب. تأكد من أنك تتوقع ذلك.
       *[other] { $count } ملف سيعمل تلقائيًا عند تسجيل الدخول إلى هذا الحاسوب. تأكد من أنك تتوقع ذلك.
    }
# $reference is a short code that names exactly this version of the plan.
plan-reference = المرجع { $reference }
# $count is how many files are affected in each line below.
plan-hand-cloud-only =
    { NUMBER($count) ->
        [zero] هناك { $count } ملف على الإنترنت فقط، ولا ينزّلها PCTwin. اجعلها متاحة على الحاسوب المحمول القديم أولًا، أو اتركها.
        [one] هناك ملف واحد على الإنترنت فقط، ولا ينزّله PCTwin. اجعله متاحًا على الحاسوب المحمول القديم أولًا، أو اتركه.
        [two] هناك ملفان على الإنترنت فقط، ولا ينزّلهما PCTwin. اجعلهما متاحين على الحاسوب المحمول القديم أولًا، أو اتركهما.
        [few] هناك { $count } ملفات على الإنترنت فقط، ولا ينزّلها PCTwin. اجعلها متاحة على الحاسوب المحمول القديم أولًا، أو اتركها.
        [many] هناك { $count } ملفًا على الإنترنت فقط، ولا ينزّلها PCTwin. اجعلها متاحة على الحاسوب المحمول القديم أولًا، أو اتركها.
       *[other] هناك { $count } ملف على الإنترنت فقط، ولا ينزّلها PCTwin. اجعلها متاحة على الحاسوب المحمول القديم أولًا، أو اتركها.
    }
plan-hand-unreadable =
    { NUMBER($count) ->
        [zero] تعذّر فتح { $count } عنصر، لذا لن تُنقل. اسمح بالوصول، أو اتركها.
        [one] تعذّر فتح عنصر واحد، لذا لن يُنقل. اسمح بالوصول، أو اتركه.
        [two] تعذّر فتح عنصرين، لذا لن يُنقلا. اسمح بالوصول، أو اتركهما.
        [few] تعذّر فتح { $count } عناصر، لذا لن تُنقل. اسمح بالوصول، أو اتركها.
        [many] تعذّر فتح { $count } عنصرًا، لذا لن تُنقل. اسمح بالوصول، أو اتركها.
       *[other] تعذّر فتح { $count } عنصر، لذا لن تُنقل. اسمح بالوصول، أو اتركها.
    }
plan-hand-work-managed =
    { NUMBER($count) ->
        [zero] هناك { $count } عنصر تديرها جهة عملك أو دراستك، لذا استُبعدت. حدّد مجلداتها لتضمينها.
        [one] هناك عنصر واحد تديره جهة عملك أو دراستك، لذا استُبعد. حدّد مجلده لتضمينه.
        [two] هناك عنصران تديرهما جهة عملك أو دراستك، لذا استُبعدا. حدّد مجلديهما لتضمينهما.
        [few] هناك { $count } عناصر تديرها جهة عملك أو دراستك، لذا استُبعدت. حدّد مجلداتها لتضمينها.
        [many] هناك { $count } عنصرًا تديرها جهة عملك أو دراستك، لذا استُبعدت. حدّد مجلداتها لتضمينها.
       *[other] هناك { $count } عنصر تديرها جهة عملك أو دراستك، لذا استُبعدت. حدّد مجلداتها لتضمينها.
    }
plan-hand-too-many-files =
    { NUMBER($count) ->
        [zero] هناك { $count } مجلد فيها ملفات أكثر مما تحمله عملية نقل واحدة، لذا لن تُنقل. بعد هذا النقل، أرسل كل مجلد وحده بالإرسال السريع، أو انسخها بمحرك أقراص USB.
        [one] هناك مجلد واحد فيه ملفات أكثر مما تحمله عملية نقل واحدة، لذا لن يُنقل. بعد هذا النقل، أرسله وحده بالإرسال السريع، أو انسخه بمحرك أقراص USB.
        [two] هناك مجلدان فيهما ملفات أكثر مما تحمله عملية نقل واحدة، لذا لن يُنقلا. بعد هذا النقل، أرسل كل مجلد وحده بالإرسال السريع، أو انسخهما بمحرك أقراص USB.
        [few] هناك { $count } مجلدات فيها ملفات أكثر مما تحمله عملية نقل واحدة، لذا لن تُنقل. بعد هذا النقل، أرسل كل مجلد وحده بالإرسال السريع، أو انسخها بمحرك أقراص USB.
        [many] هناك { $count } مجلدًا فيها ملفات أكثر مما تحمله عملية نقل واحدة، لذا لن تُنقل. بعد هذا النقل، أرسل كل مجلد وحده بالإرسال السريع، أو انسخها بمحرك أقراص USB.
       *[other] هناك { $count } مجلد فيها ملفات أكثر مما تحمله عملية نقل واحدة، لذا لن تُنقل. بعد هذا النقل، أرسل كل مجلد وحده بالإرسال السريع، أو انسخها بمحرك أقراص USB.
    }
plan-blocked-someone-unmapped = هناك شخص على الحاسوب المحمول القديم لم يُختر له حساب على الجديد بعد. اختر أين تذهب ملفات كل شخص، ثم عُد.
plan-blocked-account-used-twice = سينتهي شخصان في حساب واحد على الحاسوب المحمول الجديد. خصّص لكل شخص حسابه.
plan-blocked-invalid = لا يمكن الموافقة على هذه الخطة كما هي. ارجع وراجع ما اخترته.
plan-blocked-fix = اختيار الحسابات

## Tray menu

tray-show = إظهار PCTwin
tray-send = إرسال الملفات
tray-quit = إنهاء PCTwin

## F01 secondary actions and their coming-soon pages

welcome-restore = استعادة الإعداد الكامل
welcome-rescue = إنقاذ حاسوب محمول
welcome-quick-send = الإرسال السريع

soon-eyebrow = قريبًا
soon-restore-title = استعادة الإعداد الكامل
soon-restore-body = أعد بناء حاسوب محمول من نسخة الإعداد الكامل التي حفظتها بعد عملية نقل. هذه الميزة قادمة في إصدار لاحق.
soon-rescue-title = إنقاذ حاسوب محمول
soon-rescue-body = استرجع الملفات من حاسوب محمول لا يعمل، بتوصيل محرك أقراصه بحاسوب يعمل. هذه الميزة قادمة في إصدار لاحق.
soon-move-title = النقل الكامل
soon-move-body = انقل كل شيء من حاسوبك المحمول القديم إلى حاسوب جديد: ملفاتك وتطبيقاتك وإعداداتك. هذه الميزة قادمة في إصدار لاحق. في الوقت الحالي، استخدم الإرسال السريع لإرسال الملفات.

## X01 Move progress, compact

progress-eyebrow = عملية النقل
progress-title = جارٍ نقل أغراضك إلى هذا الحاسوب المحمول
progress-title-old = نرسل أغراضك إلى حاسوبك الجديد
progress-title-paused = النقل متوقف مؤقتًا
progress-paired = تم الربط
progress-percent =
    .aria-label = تقدّم النقل
# $done and $total are already formatted sizes, for example "81.6 GB" and "120 GB".
progress-amount = { $done } من { $total }
# $range is a measured range, already formatted, for example "15 to 20 minutes".
progress-time-left = يتبقى نحو { $range }
progress-time-measuring = جارٍ حساب الوقت المتبقي…
# $speed is measured and already formatted, for example "62 MB/s".
progress-speed = { $speed }، حسب القياس
progress-count-needs-action =
    { NUMBER($count) ->
        [zero] { $count } عنصر بحاجة إلى إجراء منك
        [one] عنصر واحد بحاجة إلى إجراء منك
        [two] عنصران بحاجة إلى إجراء منك
        [few] { $count } عناصر بحاجة إلى إجراء منك
        [many] { $count } عنصرًا بحاجة إلى إجراء منك
       *[other] { $count } عنصر بحاجة إلى إجراء منك
    }
progress-current-copying = يُنسخ الآن: { $name }
progress-current-checking = يجري التحقق الآن من: { $name }
progress-needs-action =
    { NUMBER($count) ->
        [zero] { $count } عنصر بحاجة إلى إجراء منك. ويستمر نقل الباقي.
        [one] عنصر واحد بحاجة إلى إجراء منك. ويستمر نقل الباقي.
        [two] عنصران بحاجة إلى إجراء منك. ويستمر نقل الباقي.
        [few] { $count } عناصر بحاجة إلى إجراء منك. ويستمر نقل الباقي.
        [many] { $count } عنصرًا بحاجة إلى إجراء منك. ويستمر نقل الباقي.
       *[other] { $count } عنصر بحاجة إلى إجراء منك. ويستمر نقل الباقي.
    }
progress-see-needs-action = عرض ما يحتاج إليك
progress-interrupted = انقطع الاتصال بالحاسوب المحمول الآخر. يحاول PCTwin إعادة الاتصال. لم يضِع أي شيء نُسخ وتم التحقق منه.
progress-low-battery = بطارية هذا الحاسوب منخفضة. وصّله بالكهرباء حتى لا يتوقف النقل.
progress-battery-paused = بطارية هذا الحاسوب على وشك النفاد، لذلك توقف النقل مؤقتًا بأمان. وصّله بالكهرباء ثم تابع. لم يضِع أي شيء.
progress-old-safe = يبقى حاسوبك المحمول القديم كما هو.
progress-pause = إيقاف النقل مؤقتًا
progress-compact = عرض أقل
progress-expand = عرض كل العناصر
progress-stop = إيقاف النقل

## X02 Move progress, expanded

progress-all-title = كل عناصر عملية النقل
progress-all-body = يُظهر كل عنصر من أين أتى وكيف يسير.
progress-metric-overall = التقدم الإجمالي
progress-metric-committed = تم نسخه والتحقق منه
progress-metric-estimate = تقدير
progress-col-item = العنصر
progress-col-from = من
progress-col-status = الحالة
progress-state-waiting = في الانتظار
progress-state-copying = جارٍ النسخ
progress-state-verified = تم التحقق
progress-state-checking = جارٍ التحقق
progress-state-paused = متوقف مؤقتًا
progress-checked-note = لا يُعدّ العنصر منسوخًا إلا بعد التحقق منه.
# Added 23 October 2026: T14, one tip under the progress bar, about every 12 seconds.
tips-label = نصيحة
tips-pause = إيقاف النصائح مؤقتًا
tips-resume = عرض مزيد من النصائح
tips-next = النصيحة التالية
tip-battery = يعمل هذا الحاسوب على البطارية. وصّله بالكهرباء إن أمكن، حتى لا ينقطع النقل الطويل.
tip-closer = الاتصال بطيء. قد يزيد تقريب الحاسوبين المحمولين من موجّه Wi-Fi سرعته.
tip-lids = أبقِ غطاء هذا الحاسوب مفتوحًا. قد يجعله إغلاق الغطاء يدخل في وضع السكون، فيتوقف النقل مؤقتًا.
tip-keep-using = يمكنك متابعة استخدام هذا الحاسوب. PCTwin يقرأ منه فقط.
tip-pause = يمكنك إيقاف النقل مؤقتًا في أي وقت ومتابعته لاحقًا.
tip-power-cut = إذا انقطعت الكهرباء، شغّل الحاسوبين المحمولين وافتح PCTwin. سيتابع من حيث توقف، ولن يضيع أي شيء نُسخ وتم التحقق منه.

## F09 Mini panel

mini-title = عملية النقل جارية
mini-title-paused = النقل متوقف مؤقتًا
mini-what = نقل الملفات التي حددتها
mini-progress =
    .aria-label = تقدّم النقل
mini-pause = إيقاف مؤقت
mini-resume = استئناف
mini-open = فتح عملية النقل
# Used when lane E reuses this panel for a Quick Send.
mini-title-send = جارٍ إرسال الملفات
mini-what-send = إرسال الملفات المحددة
mini-open-send = فتح الإرسال
mini-progress-send =
    .aria-label = تقدّم الإرسال

## X05 Paused at a safe point

pause-title = عملية النقل متوقفة مؤقتًا
pause-title-finishing = جارٍ إيقاف عملية النقل مؤقتًا…
pause-body = كل ما نُسخ وتم التحقق منه محفوظ. والباقي ينتظر.
pause-finishing = جارٍ إنهاء { $name } أولًا، حتى لا يبقى منسوخًا جزئيًا.
pause-kept-title = الملفات
pause-kept-body = كل ما نُسخ وتم التحقق منه حتى الآن محفوظ.
pause-waiting-title = في الانتظار
pause-waiting-body =
    { NUMBER($count) ->
        [zero] لا يزال { $count } عنصر بانتظار النسخ. لم يضِع أي شيء.
        [one] لا يزال عنصر واحد بانتظار النسخ. لم يضِع أي شيء.
        [two] لا يزال عنصران بانتظار النسخ. لم يضِع أي شيء.
        [few] لا تزال { $count } عناصر بانتظار النسخ. لم يضِع أي شيء.
        [many] لا يزال { $count } عنصرًا بانتظار النسخ. لم يضِع أي شيء.
       *[other] لا يزال { $count } عنصر بانتظار النسخ. لم يضِع أي شيء.
    }
pause-status-kept = متوقف مؤقتًا
pause-status-waiting = في الانتظار
pause-note = الإيقاف المؤقت لا يلغي أي شيء. يبقى حاسوبك المحمول القديم كما هو.
pause-resume = استئناف عملية النقل

## X10 Stop the remaining move? (danger dialog)

cancel-title = هل تريد إيقاف ما تبقى من النقل؟
cancel-body = الإيقاف يحتفظ بكل ما نُسخ وتم التحقق منه، ولا يلغي ما تم نقله.
cancel-copied-title = الملفات المنسوخة
cancel-copied-body = تبقى على الحاسوب المحمول الجديد.
cancel-rest-title = باقي عملية النقل
cancel-rest-body = لن يبدأ أي شيء جديد. الملفات التي لم تُنسخ بعد تبقى على الحاسوب المحمول القديم فقط.
cancel-partial-title = ملف نُسخ جزئيًا
cancel-partial-body = لا يُحتفظ به أبدًا كأنه مكتمل.
cancel-old-title = الحاسوب المحمول القديم
cancel-old-body = يبقى كما هو.
cancel-ack = أفهم أن الإيقاف لا يلغي ما تم نسخه
cancel-stop = الإيقاف وعرض التقرير
cancel-keep = تابع عملية النقل

## R01 Welcome to your next laptop (outcome)

outcome-eyebrow = نتائج النقل
outcome-title = انتهت عملية النقل
outcome-body = هذا ما حدث لكل ما وافقت عليه.
outcome-title-old = انتهت عملية النقل
outcome-body-old = هذا ما حدث لكل ما أرسلته. يبقى هذا الحاسوب كما كان.
outcome-files-title = الملفات المحددة
outcome-status-partial = تم جزئيًا
# Added 23 October 2026: the after-move check on R01, counted the way the scan counts.
outcome-kinds-title = ما تم التحقق منه بعد النقل
outcome-kind-photos = الصور
outcome-kind-videos = الفيديوهات
outcome-kind-music = الموسيقى
outcome-kind-documents = المستندات
outcome-kind-other = ملفات أخرى
outcome-kind-count = { $copied } من { $planned } نُسخت وتم التحقق منها
outcome-next-title = الخطوات التالية
outcome-old-safe = كل ما لم يكتمل يبقى مدرجًا هنا. لم يُحذف أي شيء من حاسوبك المحمول القديم.
outcome-landed-title = أين أغراضك الآن
outcome-report = عرض التقرير الكامل
outcome-home = العودة إلى الرئيسية
outcome-person-copied =
    { NUMBER($count) ->
        [zero] نُسخ { $count } ملف.
        [one] نُسخ ملف واحد.
        [two] نُسخ ملفان.
        [few] نُسخت { $count } ملفات.
        [many] نُسخ { $count } ملفًا.
       *[other] نُسخ { $count } ملف.
    }
outcome-person-not-done =
    { NUMBER($count) ->
        [zero] لم يكتمل { $count }.
        [one] لم يكتمل ملف واحد.
        [two] لم يكتمل ملفان.
        [few] لم تكتمل { $count } ملفات.
        [many] لم يكتمل { $count } ملفًا.
       *[other] لم يكتمل { $count } ملف.
    }
outcome-person-at-sign-in = ستكون في مكانها عندما يسجّل { $name } الدخول إلى هذا الحاسوب في المرة القادمة.
move-fallback-folder = من حاسوبك القديم
move-stop = إيقاف النقل
move-before-open = أثناء الانتظار: قبل أن تترك هذا الحاسوب
move-get-help = الحصول على مساعدة
pickup-eyebrow = عملية النقل
pickup-title = رتّب عملية النقل المتوقفة
pickup-body = توقفت عملية نقل على هذا الحاسوب في منتصفها، ربما لأنه أُعيد تشغيله أو انقطعت عنه الكهرباء. رتّب الأمور أولًا، ثم اقرن الحاسوبين من جديد للمتابعة.
pickup-unfinished-title = نسخ غير مكتملة
pickup-unfinished-body =
    { NUMBER($count) ->
        [zero] كان { $count } ملف قيد النسخ عندما توقف النقل.
        [one] كان ملف واحد قيد النسخ عندما توقف النقل.
        [two] كان ملفان قيد النسخ عندما توقف النقل.
        [few] كانت { $count } ملفات قيد النسخ عندما توقف النقل.
        [many] كان { $count } ملفًا قيد النسخ عندما توقف النقل.
       *[other] كان { $count } ملف قيد النسخ عندما توقف النقل.
    }
pickup-unfinished-badge = بحاجة إلى ترتيب
pickup-tidy = ترتيب
pickup-tidying = جارٍ الترتيب. يتم التحقق من النسخ المكتملة ووضعها في مكانها؛ ولا يتغير أي شيء يخصك.
pickup-done = اكتمل الترتيب. اربط الحاسوبين المحمولين مرة أخرى للمتابعة: ما هو موجود هنا يُعثر عليه ولا يُنسخ مرتين.
pickup-failed = لم ينجح الترتيب هذه المرة. لم يتغير أي شيء يخصك. حاول مرة أخرى، أو أغلق PCTwin على هذا الحاسوب وافتحه من جديد أولًا.
pickup-put-in-place = وُضعت في مكانها
pickup-kept = محفوظة للمتابعة
pickup-not-finished =
    { NUMBER($count) ->
        [zero] تعذّر إكمال { $count } ملف. سيُنسخ من جديد في النقل التالي.
        [one] تعذّر إكمال ملف واحد. سيُنسخ من جديد في النقل التالي.
        [two] تعذّر إكمال ملفين. سيُنسخان من جديد في النقل التالي.
        [few] تعذّر إكمال { $count } ملفات. ستُنسخ من جديد في النقل التالي.
        [many] تعذّر إكمال { $count } ملفًا. ستُنسخ من جديد في النقل التالي.
       *[other] تعذّر إكمال { $count } ملف. ستُنسخ من جديد في النقل التالي.
    }
pickup-left-for-later =
    { NUMBER($count) ->
        [zero] { $count } ملف على قرص غير متصل الآن. وصّله ثم رتّب مرة أخرى.
        [one] ملف واحد على قرص غير متصل الآن. وصّله ثم رتّب مرة أخرى.
        [two] ملفان على قرص غير متصل الآن. وصّله ثم رتّب مرة أخرى.
        [few] { $count } ملفات على قرص غير متصل الآن. وصّله ثم رتّب مرة أخرى.
        [many] { $count } ملفًا على قرص غير متصل الآن. وصّله ثم رتّب مرة أخرى.
       *[other] { $count } ملف على قرص غير متصل الآن. وصّله ثم رتّب مرة أخرى.
    }
pickup-later = ليس الآن
ready-eyebrow = من حاسوبك المحمول القديم
ready-title = أغراضك في مجلداتك
ready-body = نُقلت أشياؤك من حاسوبك القديم إلى هذا الحاسوب، وهي الآن في مجلداتك الخاصة.
ready-landed = ملف في مجلداتك
ready-where-title = أين هي
ready-where-body = في المستندات والصور والموسيقى ومقاطع الفيديو ومجلداتك المعتادة الأخرى، كما كانت على حاسوبك القديم.
ready-kept =
    { NUMBER($count) ->
        [zero] احتُفظ بـ { $count } ملف جانبًا لأن ملفًا بالاسم نفسه كان موجودًا هنا. احتُفظ بالاثنين.
        [one] احتُفظ بملف واحد جانبًا لأن ملفًا بالاسم نفسه كان موجودًا هنا. احتُفظ بالاثنين.
        [two] احتُفظ بملفين جانبًا لأن ملفات بالأسماء نفسها كانت موجودة هنا. احتُفظ بالجميع.
        [few] احتُفظ بـ { $count } ملفات جانبًا لأن ملفات بالأسماء نفسها كانت موجودة هنا. احتُفظ بالجميع.
        [many] احتُفظ بـ { $count } ملفًا جانبًا لأن ملفات بالأسماء نفسها كانت موجودة هنا. احتُفظ بالجميع.
       *[other] احتُفظ بـ { $count } ملف جانبًا لأن ملفات بالأسماء نفسها كانت موجودة هنا. احتُفظ بالجميع.
    }
ready-failed =
    { NUMBER($count) ->
        [zero] تعذّر وضع { $count } ملف في مجلداتك. اطلب من الشخص الذي أجرى النقل نقله مرة أخرى.
        [one] تعذّر وضع ملف واحد في مجلداتك. اطلب من الشخص الذي أجرى النقل نقله مرة أخرى.
        [two] تعذّر وضع ملفين في مجلداتك. اطلب من الشخص الذي أجرى النقل نقلهما مرة أخرى.
        [few] تعذّر وضع { $count } ملفات في مجلداتك. اطلب من الشخص الذي أجرى النقل نقلها مرة أخرى.
        [many] تعذّر وضع { $count } ملفًا في مجلداتك. اطلب من الشخص الذي أجرى النقل نقلها مرة أخرى.
       *[other] تعذّر وضع { $count } ملف في مجلداتك. اطلب من الشخص الذي أجرى النقل نقلها مرة أخرى.
    }
ready-done = تم
move-status-kept = محفوظ
move-wait-eyebrow = عملية النقل
move-wait-scanning-title = جارٍ البحث عن ملفاتك
move-wait-scanning-body = يتصفح حاسوبك القديم مجلداتك. إنه يقرأها فقط: لا يُفتح شيء ولا يُغيَّر ولا يُحذف.
move-found = ما وُجد حتى الآن: { $size } من أشيائك.
move-wait-approval-title = في انتظار حاسوبك الجديد
move-wait-approval-body = ملفاتك جاهزة. راجع الخطة على حاسوبك الجديد ووافق عليها للبدء.
move-wait-list-title = في انتظار حاسوبك القديم
move-wait-list-body = يبحث حاسوبك المحمول القديم عن أغراضك. ثم يُختار عليه ما سيُنقل. عند الانتهاء تظهر القائمة هنا لتقبلها. قد يستغرق ذلك بضع دقائق.
move-wait-starting-title = جارٍ بدء النقل
move-wait-starting-body = تمت الموافقة على الخطة. يبدأ النسخ بعد لحظات.
move-wait-admin-title = في انتظار موافقة المسؤول
move-wait-admin-body = يطلب هذا الحاسوب موافقة المسؤول لنقل أشياء الأشخاص الآخرين أيضًا. أجب في النافذة التي فتحها حاسوبك.
move-others-not-approved = لم يوافق المسؤول، لذلك تُنقل أشياؤك فقط هذه المرة. تبقى أشياء الأشخاص الآخرين في مكانها.
move-others-failed = تعذّر على PCTwin تشغيل ما يحتاجه لنقل أشياء الأشخاص الآخرين، لذلك تُنقل أشياؤك فقط هذه المرة. لم يتغير أي شيء يخصهم.
move-others-not-here = نقل أشياء الأشخاص الآخرين من هذا الحاسوب غير متاح بعد في هذا الإصدار، لذلك تُنقل أشياؤك فقط.
move-others-allow-in-settings = يجب أولًا أن يسمح مسؤول لـ PCTwin في صفحة إعدادات النظام التي فُتحت الآن. تُنقل أشياؤك فقط هذه المرة؛ وبعد السماح لـ PCTwin، يمكن لنقلك التالي أن يأخذ أشياء الأشخاص الآخرين أيضًا.
approve-prompt = سيقرأ PCTwin ملفات { $names } لنقلها إلى الحاسوب الجديد. يوافق مسؤول هذا الحاسوب على هذا النقل فقط.
who-here-title = من سيُنقل؟
who-here-body = تُنقل أشياؤك دائمًا. اختر أي شخص آخر على هذا الحاسوب يجب نقل أشيائه أيضًا. لا يعرض لك PCTwin ملفاتهم أبدًا.
who-here-you = { $name } (أنت)
who-here-you-hint = يُنقل دائمًا
who-here-approval = نقل أشياء شخص آخر يحتاج إلى موافقة المسؤول مرة واحدة لكل من تختارهم. سيطلبها حاسوبك في نافذته الخاصة.
who-here-not-here = نقل أشياء الأشخاص الآخرين من هذا الحاسوب غير متاح بعد في هذا الإصدار. تُنقل أشياؤك فقط.
place-title = إلى أين تذهب أشياؤهم؟
place-body = جاءت أشياء أشخاص آخرين من الحاسوب القديم أيضًا. اختر لكل شخص حسابًا على هذا الحاسوب، أو حسابًا جديدًا له، أو استبعده. لن ترى ملفاتهم أبدًا.
place-size =
    { NUMBER($count) ->
        [zero] { $count } ملف، { $size }
        [one] ملف واحد، { $size }
        [two] ملفان، { $size }
        [few] { $count } ملفات، { $size }
        [many] { $count } ملفًا، { $size }
       *[other] { $count } ملف، { $size }
    }
place-select = مكان أشياء { $name }
place-note = تحتاج الحسابات الجديدة، وحفظ أشيائهم حتى يسجلوا الدخول، إلى موافقة المسؤول مرة واحدة بعد موافقتك على الخطة. تظهر كلمة المرور المؤقتة لكل حساب جديد مرة واحدة في نافذة حاسوبك الخاصة.
place-todo =
    { NUMBER($count) ->
        [zero] اختر مكان أشياء { $count } أشخاص آخرين.
        [one] اختر مكان أشياء شخص واحد آخر.
        [two] اختر مكان أشياء شخصين آخرين.
        [few] اختر مكان أشياء { $count } أشخاص آخرين.
        [many] اختر مكان أشياء { $count } شخصًا آخر.
       *[other] اختر مكان أشياء { $count } شخص آخر.
    }
fda-title = يحتاج PCTwin إلى إذن لقراءة بعض المجلدات
fda-body =
    { NUMBER($count) ->
        [zero] تعذّرت قراءة { $count } من أماكنك بسبب إعدادات الخصوصية في جهاز Mac. امنح PCTwin الوصول ثم تحقق مرة أخرى حتى لا يفوتك أي ملف.
        [one] تعذّرت قراءة مكان واحد من أماكنك بسبب إعدادات الخصوصية في جهاز Mac. امنح PCTwin الوصول ثم تحقق مرة أخرى حتى لا يفوتك أي ملف.
        [two] تعذّرت قراءة مكانين من أماكنك بسبب إعدادات الخصوصية في جهاز Mac. امنح PCTwin الوصول ثم تحقق مرة أخرى حتى لا يفوتك أي ملف.
        [few] تعذّرت قراءة { $count } أماكن من أماكنك بسبب إعدادات الخصوصية في جهاز Mac. امنح PCTwin الوصول ثم تحقق مرة أخرى حتى لا يفوتك أي ملف.
        [many] تعذّرت قراءة { $count } مكانًا من أماكنك بسبب إعدادات الخصوصية في جهاز Mac. امنح PCTwin الوصول ثم تحقق مرة أخرى حتى لا يفوتك أي ملف.
       *[other] تعذّرت قراءة { $count } مكان من أماكنك بسبب إعدادات الخصوصية في جهاز Mac. امنح PCTwin الوصول ثم تحقق مرة أخرى حتى لا يفوتك أي ملف.
    }
fda-step-1 = افتح إعدادات النظام، ثم الخصوصية والأمن.
fda-step-2 = اختر الوصول الكامل إلى القرص.
fda-step-3 = شغّل PCTwin. قد يطلب جهاز Mac كلمة المرور.
fda-open = افتح هذه الإعدادات
fda-note = يقرأ PCTwin مجلداتك فقط لنسخها. ولا يغيّر أي شيء على هذا الحاسوب.
fda-again = تحقق مرة أخرى
fda-without = تابع بدونها
password-title = PCTwin: حساب جديد
password-body =
    أصبح لدى { $name } حساب على هذا الحاسوب.
    
    اسم تسجيل الدخول: { $account }
    كلمة المرور المؤقتة: { $password }
    
    أعطِ هذه المعلومات لـ { $name }. عند أول تسجيل دخول يختار كلمة مروره الخاصة. دوّنها الآن: لا يحتفظ بها PCTwin.
move-ended-eyebrow = عملية النقل
move-ended-kept-title = ما نُسخ بالفعل
move-ended-kept-body = كل ما نُسخ وتم التحقق منه قبل حدوث ذلك يبقى على الحاسوب الجديد.
move-ended-old-title = حاسوبك القديم
move-ended-old-body = لم يُغيَّر أو يُحذف أي شيء عليه.
move-ended-old-badge = دون تغيير
move-ended-next = للمحاولة مرة أخرى، اقرن الحاسوبين من جديد. تُكتشف الملفات المنسوخة والمتحقق منها ولا تُنسخ مرتين.
move-lost-title = انقطع الاتصال بحاسوبك الآخر
move-lost-body = توقف الحاسوبان عن الوصول إلى بعضهما، فلم يكتمل النقل. تأكد من أن كليهما يعمل وغير نائم وقريب من الآخر.
move-not-approved-title = لم يبدأ النقل
move-not-approved-old = لم تتم الموافقة على الخطة على الحاسوب الجديد، لذلك لم يُنسخ شيء.
move-not-approved-new = لم توافق على الخطة، لذلك لم يُنسخ شيء.
move-failed-title = توقفت عملية النقل بسبب خطأ
move-failed-body = تعذّر متابعة النقل. إذا تكرر ذلك، يوضح قسم مزيد من التفاصيل أدناه ما الذي حدث.
move-peer-system-windows = Windows
move-peer-system-mac = Mac
move-peer-system-linux = Linux
move-peer-system-other = نظام آخر
move-update-here-title = حدّث PCTwin على هذا الحاسوب
move-update-here-body = إصدار PCTwin على هذا الحاسوب أقدم من الحاسوب الآخر، لذا لا يمكنهما نقل الملفات معًا. حدّث PCTwin هنا ثم ابدأ من جديد. لم يُنسخ أي شيء.
move-update-there-title = حدّث PCTwin على الحاسوب الآخر
move-update-there-body = إصدار PCTwin على الحاسوب الآخر أقدم من هذا الحاسوب، لذا لا يمكنهما نقل الملفات معًا. حدّث PCTwin هناك ثم ابدأ من جديد. لم يُنسخ أي شيء.
move-refused-title = تعذّر بدء النقل
move-refused-not-paired = لم يعد حاسوباك مقترنين. اقرنهما من جديد لبدء النقل.
move-refused-busy = هناك عملية نقل جارية بالفعل على هذا الحاسوب.
move-refused-not-moving = لا توجد عملية نقل جارية على هذا الحاسوب.
move-refused-plan-changed = تغيرت الخطة قبل الموافقة عليها. اقرن الحاسوبين من جديد لرؤية الخطة الجديدة.
move-refused-failed = تعذّر تجهيز مجلدات هذا الحاسوب للنقل. اقرن الحاسوبين من جديد للمحاولة مرة أخرى.
drive-problem-title = يحتاج النقل إلى انتباهك
drive-problem-body = لم تتطابق النسخ المحفوظة في { $place } مع الأصل عندما تحقق منها PCTwin، وتكرر ذلك عدة مرات. قد يكون القرص معطلًا أو ممتلئًا أو مفصولًا، لذلك توقف النسخ إليه مؤقتًا.
drive-problem-kept-title = ما نُسخ وتم التحقق منه
drive-problem-waiting-title = ما زال يلزم نسخه إلى { $place }
drive-problem-badge = متوقف مؤقتًا
drive-problem-fix = تأكد من توصيل محرك الأقراص ووجود مساحة فارغة عليه، ثم تابع. لا يُستبعد أي شيء اخترته لتجاوز ذلك. يمكنك أيضًا إيقاف النقل.
drive-problem-carry-on = متابعة

## R02 Your move, item by item (detailed report)

report-title = ما نُقل
report-body = اختر مجموعة لترى ما فيها. ما يحتاج إليك في الأعلى.
report-needs-title = { $count ->
        [zero] يحتاج إليك: لا شيء
        [one] يحتاج إليك: واحد
        [two] يحتاج إليك: اثنان
        [few] يحتاج إليك: { $count }
        [many] يحتاج إليك: { $count }
       *[other] يحتاج إليك: { $count }
    }
report-group-photos = الصور
report-group-videos = الفيديوهات
report-group-music = الموسيقى
report-group-documents = المستندات
report-group-other = ملفات أخرى
report-group-size = { $files ->
        [zero] لا ملفات
        [one] ملف واحد
        [two] ملفان
        [few] { $files } ملفات
        [many] { $files } ملفًا
       *[other] { $files } ملف
    } · { $size }
report-group-done = نُسخ كل شيء وتم التحقق منه
report-group-some = نُسخ { $done } من { $total } وتم التحقق منها
report-group-left = { $count ->
        [zero] لا شيء لم يُنقل
        [one] واحد لم يُنقل: تركته أنت
        [two] اثنان لم يُنقلا: تركتهما أنت
        [few] { $count } لم تُنقل: تركتها أنت
        [many] { $count } لم يُنقل: تركتها أنت
       *[other] { $count } لم يُنقل: تركتها أنت
    }
report-filter-label = عرض
report-col-item = العنصر
report-col-result = النتيجة
report-col-details = التحقق أو الخطوة التالية
report-from = من: { $path }
report-to = إلى: { $path }
report-none = لا توجد عناصر بهذه النتيجة.
report-more-details = مزيد من التفاصيل
# Added 23 October 2026: what the report says beside a line's result (changed since the scan).
report-note-changed-since-scan = تغيّر منذ الفحص: نُسخ كما هو الآن.
report-note-no-longer-there = لم يعد موجودًا: نُقل أو أُعيدت تسميته أو حُذف على الحاسوب المحمول القديم قبل أن يُرسل.
report-note-time-not-kept = لم يُحفظ التاريخ: لا يستطيع هذا القرص حفظ تاريخ آخر تغيير له، لذا يظهر تاريخ نسخه.

# Evidence: how a copied item was checked.
evidence-read-back = تم التحقق بقراءته مرة أخرى من محرك الأقراص.
evidence-memory = تم التحقق عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
evidence-already-there = موجود بالفعل على الحاسوب المحمول الجديد: كان الملف نفسه هناك، لذلك لم يُنسخ مرة أخرى.
evidence-recovered = تم التحقق منه قبل إعادة تشغيل الحاسوب المحمول، ثم مرة أخرى بعدها.
evidence-new-laptop = تحقق منه الحاسوب المحمول الجديد.
evidence-memory-explain = قُرئت النسخة مرة أخرى وتطابقت، لكن عبر ذاكرة هذا الحاسوب المحمول، لذا ربما جاء ما قُرئ من الذاكرة لا من محرك الأقراص. قد لا يظهر هنا عطل في محرك الأقراص نفسه، لذا من الجيد فتح الملف مرة واحدة للتأكد.
evidence-not-checked = لم يتم التحقق، لأنه لم يكتمل.
# Machine-translated 16 October 2026.
why-memory-cannot-skip-cache = لم يسمح محرك الأقراص هذا لـ PCTwin بقراءته مباشرة، لذا تم التحقق عبر ذاكرة هذا الحاسوب المحمول. تعمل بعض محركات أقراص الشبكة وذواكر USB بهذه الطريقة.
why-memory-not-shown = لم يتمكن محرك الأقراص هذا من إظهار أن قراءة PCTwin وصلت إليه فعلًا، لذا يُحسب التحقق كأنه تم عبر ذاكرة هذا الحاسوب المحمول.
why-memory-no-way = لا يملك نظام هذا الحاسوب المحمول طريقة لقراءة محرك الأقراص هذا مباشرة، لذا تم التحقق عبر ذاكرته.
why-memory-stored-specially = هذا الملف مخزّن بطريقة خاصة، مثل الضغط أو التشفير، لذا يقرؤه النظام عبر الذاكرة.
why-memory-too-small = هذا الملف صغير جدًا، لذا يحفظه محرك الأقراص مع سجلاته الخاصة ويقرؤه النظام عبر الذاكرة.

# Reasons: why an item did not finish. Each says what happened, then what to do.
reason-not-enough-space = نفدت المساحة على الحاسوب المحمول الجديد. وفّر مساحة عليه، ثم ابدأ عملية نقل جديدة تشمل هذا العنصر.
reason-changed-during-move = ظل هذا الملف يتغير على الحاسوب المحمول القديم أثناء نسخه. أغلق أي تطبيق يستخدمه، ثم ابدأ عملية نقل جديدة تشمله.
reason-unreadable = لم يسمح الحاسوب المحمول القديم بأن يقرأ PCTwin هذا العنصر. تأكد أنه يُفتح على الحاسوب القديم، ثم ابدأ عملية نقل جديدة تشمله.
reason-not-verified = النسخة لم تطابق الأصل، لذا لم يُحتفظ بها. ابدأ عملية نقل جديدة تشمل هذا العنصر. إذا فشلت مرة أخرى، فقد يكون الأصل تالفًا.
reason-refused-by-safety-check = رفض فحص الأمان على الحاسوب المحمول الجديد هذا العنصر بسبب اسمه أو مكانه أو حجمه. لا يزال الأصل على الحاسوب القديم، لذا يمكنك نسخه بطريقة أخرى إذا احتجت إليه.
reason-name-not-allowed = لا يمكن استخدام هذا الاسم على الحاسوب المحمول الجديد، ولم يكن هناك اسم جديد آمن ممكن. غيّر اسمه على الحاسوب القديم، ثم ابدأ عملية نقل جديدة تشمله.
reason-install-failed = لم يُثبَّت التطبيق. ثبّته من موقع الجهة المطوّرة له.
reason-no-equivalent = لا يوجد إصدار من هذا التطبيق لنظام الحاسوب المحمول الجديد. ابحث عن تطبيق مشابه يعمل عليه.
reason-drive-locked = القرص الذي ينتقل إليه هذا مقفل. افتح قفله (مثلًا بكلمة مرور BitLocker أو مفتاح الاسترداد)، ثم انقله مرة أخرى.
reason-sign-in = أصبح في مكانه، لكن عليك تسجيل الدخول. افتحه وسجّل الدخول.
reason-activation = أصبح التطبيق في مكانه، لكنه يحتاج إلى ترخيصه على الحاسوب المحمول الجديد. افتحه وفعّله بترخيصك أو حسابك.
reason-permission = يجب أن يمنح الحاسوب المحمول الجديد الإذن أولًا. افتح العنصر واختر «السماح» عند السؤال.
reason-online-only = هذا الملف مخزّن على الإنترنت فقط، لذا لم يكن على الحاسوب المحمول القديم لنسخه. نزّله على الحاسوب الجديد من التطبيق الذي يخزّنه على الإنترنت.
reason-manual-install = يجب الحصول على هذا التطبيق من موقع الجهة المطوّرة له. نزّله وثبّته من هناك.
reason-restart = يجب إعادة تشغيل الحاسوب المحمول الجديد لإكمال العملية. أعد تشغيله عندما يناسبك.
reason-finish-by-hand = لا يستطيع PCTwin تنفيذ هذه الخطوة نيابةً عنك. اتبع الخطوات المعروضة لها في هذا التقرير.
reason-parts-missing = لم تصل بعض الملفات الموجودة داخل هذا المجلد، لذا فهو غير مكتمل على الحاسوب المحمول الجديد. تجدها مُدرجة في هذا التقرير مع السبب. أرسلها مرة أخرى، أو انسخ المجلد بطريقة أخرى.
reason-moved-file-missing = يشير هذا الإعداد إلى ملف أو مجلد غير موجود على الحاسوب الجديد بالاسم الذي أعطاه له النقل، لذا ترك PCTwin الإعداد كما كان. افتح التطبيق واختر الملف مرة أخرى.

## Item results (every item ends with one of these)

status-copied = تم النسخ
status-reinstalled = أُعيد التثبيت
status-converted = تم التحويل
status-needs-action = يحتاج إلى إجراء
status-skipped-by-user = تخطّيته أنت
status-failed = فشل

## Screens: words the copy deck did not cover (same voice)

# $low and $high are whole minutes, a measured range. The result fills "About { $range } left".
progress-minutes = من { $low } إلى { $high } دقيقة
# Machine-translated 16 October 2026.
progress-minutes-one =
    { NUMBER($minutes) ->
        [zero] { $minutes } دقيقة
        [one] دقيقة واحدة
        [two] دقيقتين
        [few] { $minutes } دقائق
        [many] { $minutes } دقيقة
       *[other] { $minutes } دقيقة
    }
# $low and $high are numbers and $unit is already formatted, for example "MB/s". The result fills "{ $speed }, measured".
progress-speed-range = من { $low } إلى ⁦{ $high } { $unit }⁩
progress-count-failed =
    { NUMBER($count) ->
        [zero] تعذّر نسخ { $count } عنصر
        [one] تعذّر نسخ عنصر واحد
        [two] تعذّر نسخ عنصرين
        [few] تعذّر نسخ { $count } عناصر
        [many] تعذّر نسخ { $count } عنصرًا
       *[other] تعذّر نسخ { $count } عنصر
    }
progress-queue-empty = لم يبدأ أي شيء بعد.
progress-approved-job = عملية النقل الموافق عليها
progress-route-old = الحاسوب المحمول القديم
progress-route-new = الحاسوب المحمول الجديد
progress-panel-pausing = يتوقف مؤقتًا
outcome-metric-size = تم نسخه والتحقق منه
outcome-metric-action = بحاجة إلى إجراء منك
progress-percent-unknown = لا يزال PCTwin يحسب حجم عملية النقل هذه.
mini-title-send-paused = الإرسال متوقف مؤقتًا
cancel-ack-hint = حدّد المربع أعلاه لتفعيل زر الإيقاف.
progress-state-placing = جارٍ وضعه في مكانه
progress-state-existing = موجود مسبقًا
outcome-stopped-title = تم إيقاف عملية النقل
outcome-stopped-body = لقد أوقفت عملية النقل. ما نُسخ وتم التحقق منه قبل ذلك محفوظ. والباقي لم يُنسخ.
outcome-broke-title = انتهت عملية النقل مبكرًا
outcome-broke-body = حدثت مشكلة بين الحاسوبين المحمولين، فلم تتمكن عملية النقل من الاستمرار. ما نُسخ وتم التحقق منه قبل ذلك محفوظ. والباقي لم يُنسخ.
# $done and $total are counts of items: what was copied and checked, and everything the person approved.
outcome-files-summary = العناصر التي نُسخت وتم التحقق منها: { $done } من { $total }.
outcome-files-memory =
    { NUMBER($count) ->
        [zero] تم التحقق من { $count } عنصر عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
        [one] تم التحقق من عنصر واحد عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
        [two] تم التحقق من عنصرين عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
        [few] تم التحقق من { $count } عناصر عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
        [many] تم التحقق من { $count } عنصرًا عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
       *[other] تم التحقق من { $count } عنصر عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
    }
report-show-more = عرض المزيد
# $shown and $total are counts of items.
report-showing = عرض { $shown } من { $total }

## Component kit

dialog-close = إغلاق
toast-dismiss = إخفاء
code-label = رمز من ستة أرقام

## Shared Quick Send words

qs-eyebrow = الإرسال السريع
qs-eyebrow-receiving = الاستلام على حاسوبك
qs-label-this = هذا الحاسوب
qs-label-other = الحاسوب الآخر
qs-pick-failed = تعذّر فتح نافذة الاختيار. حاول مرة أخرى.
qs-handed =
    { NUMBER($count) ->
        [zero] لم يُسلَّم أي ملف أو مجلد إلى PCTwin. راجع القائمة قبل المتابعة.
        [one] سُلِّم ملف أو مجلد واحد إلى PCTwin لإرساله. راجع القائمة قبل المتابعة.
        [two] سُلِّم ملفان أو مجلدان إلى PCTwin لإرسالهما. راجع القائمة قبل المتابعة.
        [few] سُلِّمت { $count } ملفات أو مجلدات إلى PCTwin لإرسالها. راجع القائمة قبل المتابعة.
        [many] سُلِّم { $count } ملفًا أو مجلدًا إلى PCTwin لإرسالها. راجع القائمة قبل المتابعة.
       *[other] سُلِّم { $count } ملف أو مجلد إلى PCTwin لإرسالها. راجع القائمة قبل المتابعة.
    }
qs-pick-too-many = هذه ملفات أكثر مما يحمله إرسال واحد. ألغِ تحديد بعض المجلدات، أو أرسلها على أجزاء.
qs-problem-lost-title = انقطع الاتصال بين الحاسوبين
qs-problem-failed-title = تعذّر إرسال الملفات هذه المرة
qs-problem-body = يبقى كل ما استُلم وتم التحقق منه في مكانه. اقرن الحاسوبين من جديد للمحاولة مرة أخرى.
# The route between the two laptops, for example "Blue Fox" to "Green Owl".
qs-route =
    .aria-label = من { $from } إلى { $to }
qs-route-sender = الإرسال
qs-route-receiver = استلام · شبكة Wi-Fi نفسها
qs-tally =
    { NUMBER($count) ->
        [zero] { $count } عنصر محدد · { $size }
        [one] عنصر واحد محدد · { $size }
        [two] عنصران محددان · { $size }
        [few] { $count } عناصر محددة · { $size }
        [many] { $count } عنصرًا محددًا · { $size }
       *[other] { $count } عنصر محدد · { $size }
    }
qs-tally-badge = إرسال قريب

## Q01 Quick Send home

qs-home-title = أرسل ملفات إلى حاسوب محمول قريب
qs-home-body = أو استقبل ملفات منه.
qs-send-title = إرسال الملفات
qs-send-body = اختر ملفات أو مجلدات على حاسوبك.
qs-receive-title = استلام الملفات
qs-receive-body = اسمح لحاسوب محمول قريب بإرسال ملفات إلى حاسوبك.
qs-home-action = اختيار الملفات
qs-receiving = جاهز للاستلام. ترى الحواسيب القريبة حاسوبك باسم { $label }.
qs-receive-stop = إيقاف الاستلام
qs-no-nearby = لم يُعثر على حواسيب قريبة. افتح PCTwin على الحاسوب المرسل وأبقِ الحاسوبين على شبكة Wi-Fi نفسها.

## Q02 Choose files and folders

qs-choose-title = ما الذي سترسله؟
qs-choose-body = اختر فقط الملفات والمجلدات التي تريد مشاركتها.
qs-drop =
    .aria-label = منطقة إفلات الملفات والمجلدات
qs-drop-title = أفلِت الملفات أو المجلدات هنا
qs-drop-body = أو أضفها عبر «إضافة ملفات» أو «إضافة مجلدات».
qs-drop-active = أفلِتها لإضافتها.
qs-add-files = إضافة ملفات
qs-add-folders = إضافة مجلدات
qs-remove =
    .aria-label = إزالة { $name }
# $kind is the file type, for example "PDF document" or "8 photos".
qs-file-detail = { $size } · { $kind }
qs-empty = لم يُختر أي شيء بعد. أضف ملفات أو مجلدات لإرسالها.
qs-online-only = بعض الملفات متاحة عبر الإنترنت فقط. اجعلها متاحة على حاسوبك أولًا، ثم أرسلها.
qs-protected = لا يستطيع PCTwin قراءة { $name }. اسمح بالوصول، أو أزله من القائمة.
qs-allow = السماح بالوصول
qs-choose-continue = متابعة

## Q12 Send progress

qs-sending-title = جارٍ إرسال ملفاتك
qs-sending-body = تذهب النسخ مباشرة إلى الحاسوب المستلم.
qs-sending-progress =
    .aria-label = إرسال الملفات
qs-sending-current = يُرسل الآن: { $name }
# $speed is measured and already formatted, for example "3.1 MB/s".
qs-sending-speed = { $speed }، حسب القياس
qs-step-copying = النسخ
qs-step-copying-body =
    { NUMBER($done) ->
        [zero] أُرسل { $done } ملف.
        [one] أُرسل ملف واحد.
        [two] أُرسل ملفان.
        [few] أُرسلت { $done } ملفات.
        [many] أُرسل { $done } ملفًا.
       *[other] أُرسل { $done } ملف.
    }
qs-step-checking = التحقق
qs-step-checking-body = يُقارن كل ملف بالأصل.
qs-sending-paused = متوقف مؤقتًا. الملفات التي استُلمت وتم التحقق منها محفوظة.
qs-sending-disconnected = انقطع الاتصال مع { $label }. الملفات التي تم التحقق منها محفوظة. تجري إعادة المحاولة تلقائيًا…
qs-sending-needs-action =
    { NUMBER($count) ->
        [zero] { $count } عنصر بحاجة إلى إجراء منك.
        [one] عنصر واحد بحاجة إلى إجراء منك.
        [two] عنصران بحاجة إلى إجراء منك.
        [few] { $count } عناصر بحاجة إلى إجراء منك.
        [many] { $count } عنصرًا بحاجة إلى إجراء منك.
       *[other] { $count } عنصر بحاجة إلى إجراء منك.
    }
qs-pause = إيقاف الإرسال مؤقتًا
qs-resume = استئناف
qs-compact = عرض أقل
qs-stop = إيقاف هذا الإرسال
qs-sending-complete-title = اكتمل الإرسال

## Q20 Stop this send? (and decline)

qs-stop-title = هل تريد إيقاف هذا الإرسال؟
qs-stop-body = تحقق مما وصل بالفعل قبل أن توقف الإرسال.
qs-stop-arrived-title = الملفات التي وصلت كاملة
qs-stop-arrived-body = تبقى في المجلد الذي اخترته.
qs-stop-rest-title = الملفات التي لم تُرسل بعد
qs-stop-rest-body = لا يُرسل أي شيء جديد بعد الإيقاف.
qs-stop-partial-title = الملفات التي وصلت جزئيًا
qs-stop-partial-body = لا تُحفظ أبدًا باسمها الحقيقي، فلا يظنها أحد نسخًا كاملة.
qs-stop-status-keep = الاحتفاظ
qs-stop-status-stop = إيقاف
qs-stop-status-review = مراجعة
qs-stop-ack = إيقاف هذا الإرسال
qs-stop-ack-hint = الملفات التي وصلت كاملة لا تُحذف.
qs-stop-keep = متابعة الإرسال

## F08 Your moves, kept on this laptop (history)

history-title = سجل عمليات النقل
history-body = افتح تقريرًا مرة أخرى، أو تابع عملية نقل محفوظة.
history-row-move = ‏{ $from } ← { $to }
history-status-finished = مكتمل
history-status-steps-left =
    { NUMBER($count) ->
        [zero] مكتمل، وتبقى عليك { $count } خطوة
        [one] مكتمل، وتبقى عليك خطوة واحدة
        [two] مكتمل، وتبقى عليك خطوتان
        [few] مكتمل، وتبقى عليك { $count } خطوات
        [many] مكتمل، وتبقى عليك { $count } خطوة
       *[other] مكتمل، وتبقى عليك { $count } خطوة
    }
history-status-paused = متوقف مؤقتًا
history-status-stopped = متوقف
history-open-report = التقرير
history-undo = تراجع
history-continue = متابعة
history-export = تصدير السجل
history-exported = حُفظ السجل في { $path }.
history-export-failed = تعذّر حفظ السجل هناك. اختر مجلدًا آخر وحاول مرة أخرى.
history-note = يبقى هذا السجل على هذا الحاسوب المحمول. لا يُرسل أي شيء إلى أي مكان إلا إذا صدّرته وشاركته بنفسك.
history-empty = ستظهر هنا أول عملية نقل لك وتقريرها.
history-empty-title = لا توجد عمليات نقل مسجلة بعد
history-start = بدء عملية نقل جديدة

## Quick Send: words the mockups do not have (added while building; same voice)

qs-soon-note = الإرسال السريع غير مفعّل في هذا الإصدار بعد. لم يُرسل أو يُستلم أي شيء.
qs-tally-counting =
    { NUMBER($count) ->
        [zero] { $count } عنصر محدد · لا يزال الحجم قيد الحساب
        [one] عنصر واحد محدد · لا يزال الحجم قيد الحساب
        [two] عنصران محددان · لا يزال الحجم قيد الحساب
        [few] { $count } عناصر محددة · لا يزال الحجم قيد الحساب
        [many] { $count } عنصرًا محددًا · لا يزال الحجم قيد الحساب
       *[other] { $count } عنصر محدد · لا يزال الحجم قيد الحساب
    }
qs-gone = لم يعد { $name } موجودًا. ربما نُقل أو حُذف. ألغِ تحديده، أو أعده إلى مكانه وأضفه مرة أخرى.
qs-choose-why-problems = لا يمكن إرسال بعض العناصر المحددة بعد. ألغِ تحديدها، أو أصلحها كما هو موضح، للمتابعة.
qs-sending-card = إرسال الملفات
qs-progress-of = من { $total }
history-open-name = { $what }: { $title }

## R05 التراجع: ما سيفعله التراجع على كل محرك أقراص، وما انتهت إليه كل نسخة

undo-here-removes = يحذف التراجع النُسخ التي أنشأها PCTwin ولم تغيّرها. يتم الاحتفاظ بكل ما غيّرته.
undo-here-to-trash = ينقل التراجع النُسخ التي أنشأها PCTwin ولم تغيّرها إلى سلة المهملات. يتم الاحتفاظ بكل ما غيّرته، ولا يُحذف أي شيء.
undo-here-keeps-fat = عند التراجع، سيتم الاحتفاظ بكل نسخة على { $drive }. يستخدم محرك الأقراص هذا FAT أو exFAT، وهما يعيدان استخدام الأرقام التي تميّز الملفات، لذلك لا يستطيع PCTwin أن يتأكد أيّ الملفات تخصّه. يمكنك حذفها بنفسك.
undo-here-keeps-other = عند التراجع، سيتم الاحتفاظ بكل نسخة على { $drive }. لا يستطيع PCTwin نقل الملفات أو حذفها بأمان على هذا النوع من محركات الأقراص. يمكنك حذفها بنفسك.
undo-here-more = مزيد من التفاصيل
undo-here-more-trash = لا يتيح جهاز Mac لـ PCTwin أن يتأكد من أن أي تطبيق آخر لم يعد يكتب في الملف، لذلك لا يحذف PCTwin أي ملف أبدًا. بل ينقل كل نسخة إلى سلة المهملات على محرك الأقراص نفسه. يمكنك إرجاعها من سلة المهملات.
undo-here-more-fat = على محركات الأقراص FAT وexFAT، قد يحصل ملف جديد على الرقم الذي كان لملف محذوف. لا يتعامل PCTwin إلا مع الملفات التي يستطيع إثبات أنها تخصّه وأنها لم تتغيّر، لذلك يتم الاحتفاظ بهذه الملفات.
undo-done-trashed = نُقل إلى سلة المهملات باسم "{ $name }". يمكنك إرجاعه من سلة المهملات.
undo-kept-fat = تم الاحتفاظ به. يعيد محرك الأقراص هذا (FAT أو exFAT) استخدام الأرقام التي تميّز الملفات، لذلك لا يستطيع PCTwin أن يتأكد أن هذه نسخته. إذا لم تكن بحاجة إليها، يمكنك حذفها بنفسك.
undo-kept-no-trash = تم الاحتفاظ به. لم يتمكن PCTwin من نقل هذه النسخة إلى سلة المهملات على محرك الأقراص الخاص بها. إذا لم تكن بحاجة إليها، يمكنك حذفها بنفسك.

## Quick Send (26 October 2026): pairing in Quick Send's words, where received files go, and the Done page

qs-pair-old-eyebrow = الإرسال
qs-pair-old-body = على الحاسوب المستلم، اختر «استلام الملفات»، ثم أدخل هذا الرمز.
qs-pair-old-waiting = في انتظار الحاسوب المستلم…
qs-pair-old-label-hint = سيجد الحاسوب المستلم حاسوبك بهذا الاسم.
qs-pair-old-steps-title = على الحاسوب المستلم
qs-pair-old-step-1 = افتح PCTwin، واختر «الإرسال السريع»، ثم «استلام الملفات».
qs-pair-old-left = أوقف الحاسوب المستلم الربط قبل اختيار رقم، لذا لم تتم مشاركة أي شيء. هذا رمز جديد.
qs-pair-network-hint = اختر الشبكة التي يتصل بها الحاسوب المستلم.
qs-pair-choose-body = انظر إلى الحاسوب المستلم. اختر الرقم الذي يعرضه.
qs-pair-choose-renewed = ظهرت أرقام جديدة. اختر الرقم الظاهر على الحاسوب المستلم.
qs-pair-choose-note = إذا لم يعرض الحاسوب المستلم أي رقم، أو لم يطابقه أي من هذه الأرقام، فاختر «لا يطابق أي منها».
qs-pair-heads-up-firewall = قد يسألك Windows هل يمكن لـ PCTwin قبول الاتصالات. اختر «السماح» حتى يتمكن الحاسوب المستلم من الوصول إلى حاسوبك.
qs-pair-new-eyebrow = الاستلام
qs-pair-new-searching-title = جارٍ البحث عن الحاسوب المرسل…
qs-pair-new-searching-body = أبقِ PCTwin مفتوحًا على الحاسوب المرسل، وعلى شبكة Wi-Fi نفسها التي يتصل بها حاسوبك.
qs-pair-new-search-failed-title = تعذّر البحث عن الحاسوب المرسل
qs-pair-new-list-title = أي حاسوب يرسل؟
qs-pair-new-list-body = اختر الاسم الذي يعرضه الحاسوب المرسل على شاشته.
qs-pair-new-same-label = حاسوب آخر قريب يحمل هذا الاسم. تحقق من شاشة الحاسوب المرسل.
qs-pair-new-none-title = لم يُعثر على حاسوب يرسل
qs-pair-new-none-body = تأكد أن PCTwin مفتوح على الحاسوب المرسل ويعرض رمزًا، وأن الحاسوبين على شبكة Wi-Fi نفسها. ثم ابحث مرة أخرى.
qs-pair-new-code-hint = اقرأه من شاشة { $label }. يتغير كل دقيقة.
qs-pair-new-number-title = اختر هذا الرقم على { $label }
qs-pair-new-number-body = تظهر ثلاثة أرقام على { $label }. اختر هذا الرقم هناك.
qs-pair-new-number-waiting = في انتظار { $label }…
qs-pairing-code-invalid = يجب أن يتكون الرمز من ستة أرقام. تحقق من شاشة { $label } واكتبه مرة أخرى.
qs-pairing-code-wrong = الرمز غير مطابق. تحقق من شاشة { $label } وحاول مرة أخرى.
qs-pairing-code-expired = تغيّر الرمز. اكتب الرمز الجديد من شاشة { $label }.
qs-pairing-busy = لا يمكن الربط مع { $label } الآن لأنه يرتبط بحاسوب آخر. انتظر لحظة، ثم حاول مرة أخرى.
qs-pairing-wait = يجري تجهيز رمز جديد على { $label }. انتظر حتى يظهر، ثم اكتب الرمز الجديد.
qs-pairing-locked = توقف الربط بعد محاولات خاطئة كثيرة. على { $label }، اختر «البدء من جديد».
qs-pairing-unreachable = تعذّر على حاسوبك الوصول إلى { $label }. تأكد أن الحاسوبين على شبكة Wi-Fi نفسها، ثم حاول مرة أخرى.
qs-pairing-wrong-number = هذا الرقم لا يطابق رقم الحاسوب المستلم. لم تتم مشاركة أي شيء. اكتب الرمز الجديد على الحاسوب المستلم لتحاول مرة أخرى.
qs-pairing-no-match = لم تتطابق الأرقام على { $label }. لم تتم مشاركة أي شيء. اكتب الرمز الجديد من شاشته.
qs-pairing-timed-out = لم يُختر أي رقم في الوقت المحدد. حاول مرة أخرى بالرمز الجديد على { $label }.
qs-pairing-connection-lost-new = انقطع الاتصال بـ { $label }. لم تتم مشاركة أي شيء.
qs-pairing-choose-network = حاسوبك متصل بأكثر من شبكة. اختر الشبكة التي يتصل بها الحاسوب المستلم:
qs-pairing-failed-new = لم ينجح الاقتران هذه المرة. لم تتم مشاركة أي شيء. اكتب الرمز الجديد من شاشة { $label }.
qs-folder-line = ستُحفظ الملفات في { $folder }.
qs-folder-change = تغيير
qs-folder-title = أين تُحفظ الملفات؟
qs-folder-none = لا يمكن لـ PCTwin استخدام مجلد الملفات المستلمة. اختر مجلدًا للملفات، ثم اتصل.
qs-folder-choose = اختيار مجلد
qs-folder-failed = لا يمكن لـ PCTwin حفظ الملفات في هذا المجلد. اختر مجلدًا آخر.
qs-done-sent-title = أُرسلت ملفاتك
qs-done-here-title = تم استلام الملفات
qs-done-partial-title = لم تصل بعض الملفات
qs-done-stopped-title = أُوقف هذا الإرسال
qs-done-empty-title = لم يُرسل أي شيء
qs-done-arrived =
    { NUMBER($count) ->
        [zero] { $count } ملف · { $size } وصلت ومطابقة للأصل
        [one] ملف واحد · { $size } وصل ومطابق للأصل
        [two] ملفان · { $size } وصلا ومطابقان للأصل
        [few] { $count } ملفات · { $size } وصلت ومطابقة للأصل
        [many] { $count } ملفًا · { $size } وصلت ومطابقة للأصل
       *[other] { $count } ملف · { $size } وصلت ومطابقة للأصل
    }
qs-done-arrived-count =
    { NUMBER($count) ->
        [zero] { $count } ملف وصلت ومطابقة للأصل
        [one] ملف واحد وصل ومطابق للأصل
        [two] ملفان وصلا ومطابقان للأصل
        [few] { $count } ملفات وصلت ومطابقة للأصل
        [many] { $count } ملفًا وصلت ومطابقة للأصل
       *[other] { $count } ملف وصلت ومطابقة للأصل
    }
qs-done-already =
    { NUMBER($count) ->
        [zero] { $count } ملف كانت موجودة مسبقًا ومطابقة، لذا لم تُرسل مرة أخرى.
        [one] ملف واحد كان موجودًا مسبقًا ومطابقًا، لذا لم يُرسل مرة أخرى.
        [two] ملفان كانا موجودين مسبقًا ومطابقين، لذا لم يُرسلا مرة أخرى.
        [few] { $count } ملفات كانت موجودة مسبقًا ومطابقة، لذا لم تُرسل مرة أخرى.
        [many] { $count } ملفًا كانت موجودة مسبقًا ومطابقة، لذا لم تُرسل مرة أخرى.
       *[other] { $count } ملف كانت موجودة مسبقًا ومطابقة، لذا لم تُرسل مرة أخرى.
    }
qs-done-stopped-body = الملفات التي وصلت قبل الإيقاف تم التحقق منها وتبقى حيث حُفظت. لم يُرسل أي شيء آخر.
qs-done-folder = حُفظت في { $folder }
qs-done-open-folder = فتح المجلد
qs-done-dates-new =
    { NUMBER($count) ->
        [zero] لا توجد ملفات جديدة.
        [one] يظهر هذا الملف بوقت وصوله، لذا فهو في أعلى المجلد.
        [two] يظهر هذان الملفان بوقت وصولهما، لذا فهما في أعلى المجلد.
        [few] تظهر هذه الملفات الـ{ $count } بوقت وصولها، لذا فهي في أعلى المجلد.
        [many] تظهر هذه الملفات الـ{ $count } بوقت وصولها، لذا فهي في أعلى المجلد.
       *[other] تظهر هذه الملفات الـ{ $count } بوقت وصولها، لذا فهي في أعلى المجلد.
    }
qs-done-keep-dates = الاحتفاظ بالتواريخ الأصلية
qs-done-dates-kept =
    { NUMBER($count) ->
        [zero] لم يُعَد أي تاريخ أصلي.
        [one] عاد إلى ملف واحد تاريخه الأصلي.
        [two] عاد إلى ملفين تاريخهما الأصلي.
        [few] عاد إلى { $count } ملفات تاريخها الأصلي.
        [many] عاد إلى { $count } ملفًا تاريخه الأصلي.
       *[other] عاد إلى { $count } ملف تاريخه الأصلي.
    }
qs-done-dates-failed = تعذّرت إعادة التواريخ الأصلية. الملفات سليمة وتبقى في مكانها.
qs-done-open-failed = تعذّر فتح المجلد. ربما نُقل أو حُذف.
qs-done-not-opened = لم يفتح PCTwin أي ملف مستلم ولم يشغّله.
qs-done-problems-title = ملفات تحتاج إلى انتباه
qs-done-not-arrived = لم يصل.
qs-done-not-completed = لم يُرسل قبل الإيقاف.
qs-done-needs-action = إنه هنا، لكنه لم يكتمل. ما يجب فعله موجود في مزيد من التفاصيل.
qs-done-mark-not-kept = إنه هنا، لكن هذا القرص لم يتمكن من تمييزه كملف تم تنزيله من حاسوب آخر. افتحه بحذر.
qs-done-see-files = عرض الملفات
qs-done-renamed = حُفظ باسم { $name }، لأن ملفًا بهذا الاسم كان موجودًا مسبقًا.
qs-done-identical = كان ملف مطابق موجودًا مسبقًا، لذا لم يُرسل مرة أخرى.
qs-done-send-missing = إرسال الملفات الناقصة
qs-done-send-missing-body = الملفات التي لم تصل فقط. تربط الحاسوبين مرة أخرى أولًا.
qs-done-done = تم
qs-stop-now = إيقاف الإرسال
qs-stop-finished = انتهى هذا الإرسال بالفعل.
qs-route-receiving = الاستلام
qs-receiving-title = جارٍ استلام الملفات
qs-receiving-body = تأتي النسخ مباشرة من { $label }.
qs-receiving-card = استلام الملفات
qs-receiving-progress =
    .aria-label = استلام الملفات
qs-receiving-current = جارٍ الاستلام: { $name }
qs-step-receiving-body =
    { NUMBER($done) ->
        [zero] { $done } ملف وصلت.
        [one] وصل ملف واحد.
        [two] وصل ملفان.
        [few] { $done } ملفات وصلت.
        [many] { $done } ملفًا وصلت.
       *[other] { $done } ملف وصلت.
    }
qs-tally-files =
    { NUMBER($count) ->
        [zero] { $count } ملف · { $size }
        [one] ملف واحد · { $size }
        [two] ملفان · { $size }
        [few] { $count } ملفات · { $size }
        [many] { $count } ملفًا · { $size }
       *[other] { $count } ملف · { $size }
    }
qs-receive-pause = إيقاف مؤقت
qs-label-yours = حاسوبك
qs-label-receiving = الحاسوب المستلم
qs-label-sending = الحاسوب المرسل
qs-peer-receiving = الحاسوب المستلم
qs-peer-sending = الحاسوب المرسل
qs-pair-old-label-title = اسم حاسوبك
qs-pair-old-rename-label = اسم لحاسوبك
qs-pair-old-step-2 = إذا ظهرت قائمة، فاختر اسم حاسوبك.
qs-pair-old-locked-body = بعد خمسة رموز خاطئة، توقف الربط لحماية حاسوبك. لم تتم مشاركة أي شيء. إذا كنت أنت، فاختر «البدء من جديد» للحصول على رمز جديد.
qs-pair-new-needs-update = عليه إصدار أقدم من PCTwin. حدّث PCTwin عليه، ثم ابحث مرة أخرى.
qs-pairing-no-network = حاسوبك غير متصل بشبكة Wi-Fi أو سلكية. صِله بشبكة، ثم حاول مرة أخرى.
qs-problem-room-title = لا توجد مساحة كافية في { $folder }
qs-problem-room-body = تحتاج هذه الملفات إلى { $needed }، ويمكن استخدام { $room } هناك. لم يُحفظ أي شيء. أفرغ بعض المساحة أو اختر مجلدًا آخر، ثم حاول مرة أخرى.
qs-done-refused-title = لم يقبل الحاسوب المستلم هذه الملفات
qs-done-refused-body = ربما لا توجد عليه مساحة كافية. لم يُرسل أي شيء.

## شاشات المرحلة 2 (قائمة المهام 2.8): الأجزاء المشتركة

guard-state-not-started = لم تبدأ بعد
guard-state-preparing = جارٍ التحضير
guard-state-awaiting-person = في انتظار اختيارك
guard-state-awaiting-system = في انتظار موافقة نظامك
guard-state-awaiting-maker = في انتظار مطوّر التطبيق أو الخدمة عبر الإنترنت
guard-state-working = جارٍ العمل
guard-state-checking = جارٍ التحقق مما تم
guard-state-applying = جارٍ وضع الأشياء في أماكنها
guard-state-pause-requested = سيتوقف مؤقتًا بعد الخطوة الحالية. لم يتوقف بعد.
guard-state-paused = متوقف مؤقتًا
guard-state-source-unavailable = الحاسوب المحمول القديم أو محرك الأقراص غير متصل
guard-state-blocked = هناك أمر يجب إصلاحه أولًا
guard-state-partial = تم جزء منه
guard-state-retrying = جارٍ إعادة المحاولة
guard-state-reconciling = جارٍ التحقق مما اكتمل
guard-state-failed = لم ينجح هذا
guard-state-complete = تم
guard-state-cancelled = تم الإيقاف. ما تم إنجازه من قبل محفوظ.
guard-refused-unavailable = لم يتمكن PCTwin من التحقق من هذا الآن. لم يتم شيء. أعد المحاولة بعد قليل.
guard-refused-wrong-job = كان هذا لعملية نقل أخرى. لم يتم شيء.
guard-refused-old-session = كانت هذه الشاشة قديمة، لذلك لم يتم شيء. هذا هو الوضع الصحيح الآن.
guard-refused-plan-changed = تغيّرت الخطة أثناء نظرك إليها، لذلك لم يتم شيء. ألقِ نظرة عليها مرة أخرى.
guard-refused-not-now = لا يمكن فعل ذلك في هذه الخطوة. لم يتم شيء.
guard-refused-not-shown = هذا الخيار لم يعد في هذه الشاشة. لم يتم شيء.
guard-progress-label = التقدم
guard-progress-mark = اكتمل { $percent }%
guard-progress-of = { $done } من { $total }
guard-progress-unknown-total = { $done } حتى الآن. العدد الإجمالي غير معروف بعد.
guard-progress-none = لم يُقَس أي شيء بعد.
guard-and-more =
    { $count ->
        [zero] و{ $count } أخرى، مذكورة في التقرير.
        [one] وعنصر واحد آخر، مذكور في التقرير.
        [two] وعنصران آخران، مذكوران في التقرير.
        [few] و{ $count } عناصر أخرى، مذكورة في التقرير.
        [many] و{ $count } عنصرًا آخر، مذكورة في التقرير.
       *[other] و{ $count } عنصر آخر، مذكورة في التقرير.
    }
guard-not-known-yet = غير معروف بعد
guard-free = { $free } متاحة
guard-free-unknown = المساحة الحرة غير معروفة
guard-items-size = { $items } عناصر، { $size }
guard-try-again = إعادة المحاولة
guard-pause = إيقاف مؤقت
guard-resume = استئناف
guard-check-again = التحقق مرة أخرى
guard-strip-plan = خطتك
guard-strip-report = تقريرك
guard-strip-permission = الموافقة
guard-strip-rescue = الإنقاذ
guard-strip-undo = التراجع
guard-strip-checking = التحقق

## X09 التحقق بعد إعادة التشغيل، وعدم توفر الحالة

guard-checking-title = جارٍ التحقق مما اكتمل
guard-checking-body = يفحص PCTwin ما كان قد اكتمل قبل أن يُغلق. ستعود خياراتك فور أن يعرف ذلك.
guard-unavailable-title = الحالة غير متاحة
guard-unavailable-body = لا يستطيع PCTwin أن يقول الآن ما الذي تفعله عملية النقل هذه. لن يظهر أي شيء على أنه تم حتى يستطيع ذلك. تحقق مرة أخرى بعد قليل.

## 2.9 قبل أن نغيّر أي شيء

guard-changes-title = ما الذي ستغيّره عملية النقل
guard-changes-body = هذا كل ما ستغيّره عملية النقل على هذا الحاسوب المحمول، وما يستطيع التراجع فعله مع كل شيء. لم يتغيّر أي شيء بعد.
guard-changes-download = التنزيل من الإنترنت
guard-changes-download-none = لا شيء
guard-changes-permissions = موافقة النظام
guard-changes-one-prompt = يسأل نظامك مرة واحدة
guard-changes-no-prompt = غير مطلوبة
guard-changes-restore-point = نقطة استعادة في Windows
guard-changes-restore-point-first = تُطلب أولًا
guard-changes-restore-point-none = غير مستخدمة
guard-changes-install =
    { $count ->
        [zero] للتثبيت: لا تطبيقات
        [one] للتثبيت: تطبيق واحد
        [two] للتثبيت: تطبيقان
        [few] للتثبيت: { $count } تطبيقات
        [many] للتثبيت: { $count } تطبيقًا
       *[other] للتثبيت: { $count } تطبيق
    }
guard-changes-merge =
    { $count ->
        [zero] تُضاف بجانب ما هو موجود هنا: لا عناصر
        [one] يُضاف بجانب ما هو موجود هنا: عنصر واحد
        [two] يُضافان بجانب ما هو موجود هنا: عنصران
        [few] تُضاف بجانب ما هو موجود هنا: { $count } عناصر
        [many] تُضاف بجانب ما هو موجود هنا: { $count } عنصرًا
       *[other] تُضاف بجانب ما هو موجود هنا: { $count } عنصر
    }
guard-changes-replace =
    { $count ->
        [zero] للاستبدال مع الاحتفاظ بالنسخ القديمة: لا عناصر
        [one] للاستبدال مع الاحتفاظ بالنسخة القديمة: عنصر واحد
        [two] للاستبدال مع الاحتفاظ بالنسختين القديمتين: عنصران
        [few] للاستبدال مع الاحتفاظ بالنسخ القديمة: { $count } عناصر
        [many] للاستبدال مع الاحتفاظ بالنسخ القديمة: { $count } عنصرًا
       *[other] للاستبدال مع الاحتفاظ بالنسخ القديمة: { $count } عنصر
    }
guard-changes-leave-alone =
    { $count ->
        [zero] تبقى كما هي: لا عناصر
        [one] يبقى كما هو: عنصر واحد
        [two] يبقيان كما هما: عنصران
        [few] تبقى كما هي: { $count } عناصر
        [many] تبقى كما هي: { $count } عنصرًا
       *[other] تبقى كما هي: { $count } عنصر
    }
guard-changes-space = المساحة المطلوبة على كل محرك أقراص في أكثر لحظة ازدحامًا
guard-changes-why-prompt = لماذا سيسأل نظامك
guard-changes-approve = الموافقة على هذه التغييرات
guard-undo-legend-can = يستطيع PCTwin التراجع عنه.
guard-undo-legend-keeps = يحتفظ به PCTwin. لا يمسّه التراجع.
guard-undo-legend-you = يحتاج إليك. لا يستطيع التراجع عنه إلا خدمته أو مطوّره.
guard-undo-can-reverse = يستطيع PCTwin التراجع عنه
guard-undo-can-remove-app = يستطيع PCTwin إزالة التطبيق. ولا يلمس الملفات التي أنشأتها.
guard-undo-must-keep = محفوظ: كان موجودًا هنا من قبل
guard-undo-needs-you-service = يحتاج إليك: لا تستطيع التراجع عنه إلا الخدمة عبر الإنترنت
guard-undo-needs-you-maker = يحتاج إليك: تسجيلات الدخول والتراخيص تعود إلى المطوّر
guard-drive-needs = يحتاج إلى { $needed }. المتاح { $free }.
guard-drive-needs-free-unknown = يحتاج إلى { $needed }. لا يذكر محرك الأقراص كم بقي فيه من مساحة.
guard-drive-fits = يتّسع
guard-drive-short = ينقصه { $short }
guard-drive-unknown = غير معروف
guard-permission-read-other-people = لقراءة ملفات الأشخاص الآخرين على الحاسوب المحمول القديم
guard-permission-make-accounts = لإنشاء حساباتهم على هذا الحاسوب المحمول
guard-permission-shared-folders = لوضع أشياء في مجلدات مشتركة لا يستطيع حسابك تغييرها
guard-permission-install-for-everyone = لتثبيت التطبيقات للجميع على هذا الحاسوب المحمول
guard-permission-restore-point = لإنشاء نقطة استعادة في Windows أولًا

## X06 وH10 تغيّر شيء بعد الموافقة

guard-changed-title =
    { $count ->
        [zero] لم يتغيّر أي شيء بعد موافقتك
        [one] تغيّر شيء واحد بعد موافقتك
        [two] تغيّر شيئان بعد موافقتك
        [few] تغيّرت { $count } أشياء بعد موافقتك
        [many] تغيّر { $count } شيئًا بعد موافقتك
       *[other] تغيّر { $count } شيء بعد موافقتك
    }
guard-changed-body = هذه فقط تحتاج إلى مراجعة جديدة. كل ما عداها يبقى موافقًا عليه ويستمر.
guard-changed-changed-on-old-laptop = تغيّر على الحاسوب المحمول القديم.
guard-changed-app-updated = حدّث التطبيق نفسه بنفسه.
guard-changed-destination-changed = تغيّر المكان الذي يذهب إليه.
guard-changed-new-scope = إنه جديد: انضم شخص آخر أو حاسوب محمول آخر إلى عملية النقل.
guard-changed-permission-ended = انتهت الموافقة الخاصة به.
guard-changed-leave-out = استبعاد هذه
guard-changed-approve = الموافقة على هذه مرة أخرى

## M31 وM05 المساحة على الحاسوب المحمول الجديد

guard-space-title = المساحة على الحاسوب المحمول الجديد
guard-space-title-short = المساحة على الحاسوب المحمول الجديد غير كافية
guard-space-body = حسب PCTwin كل ما سيصل إلى كل محرك أقراص، بما في ذلك المساحة اللازمة للتراجع. لا يُسقَط أي شيء اخترته من أجل أن يتّسع.
guard-space-proposal = ما يقترحه PCTwin لما لا يتّسع
guard-keep-off-leave-in-cloud = تُترك على الإنترنت، حيث هي مخزّنة الآن
guard-keep-off-offload-drive = تُوضع على محرك الأقراص الآخر لديك
guard-keep-off-leave-on-old-laptop = تُترك على الحاسوب المحمول القديم. احتفظ به حتى يتم نقلها.
guard-offload-title = محرك الأقراص الآخر لديك
guard-offload-connected = متصل
guard-offload-not-connected = غير متصل
guard-offload-approved = اخترت محرك الأقراص هذا. الملفات الموجودة عليه ليست على الحاسوب المحمول الجديد: وصّله لفتحها.
guard-offload-ask = يمكن لـ PCTwin وضع بعض الأشياء على محرك الأقراص هذا. يحتاج إلى موافقتك على محرك الأقراص هذا بشكل منفصل.
guard-offload-use = استخدام محرك الأقراص هذا
guard-space-approve = الموافقة على هذه الخطة
guard-safe-in = مخزّنة في { $provider }
guard-safe-in-cloud = مخزّنة على الإنترنت
guard-safe-in-detail =
    { $count ->
        [zero] لا توجد ملفات محفوظة على الإنترنت فقط.
        [one] ملف واحد محفوظ على الإنترنت فقط. سيعود عندما تسجّل الدخول على الحاسوب المحمول الجديد.
        [two] ملفان محفوظان على الإنترنت فقط. سيعودان عندما تسجّل الدخول على الحاسوب المحمول الجديد.
        [few] { $count } ملفات محفوظة على الإنترنت فقط. ستعود عندما تسجّل الدخول على الحاسوب المحمول الجديد.
        [many] { $count } ملفًا محفوظة على الإنترنت فقط. ستعود عندما تسجّل الدخول على الحاسوب المحمول الجديد.
       *[other] { $count } ملف محفوظة على الإنترنت فقط. ستعود عندما تسجّل الدخول على الحاسوب المحمول الجديد.
    }
guard-your-cloud = مساحة تخزينك على الإنترنت

## M09 الاحتفاظ بنسخة من الملفات الموجودة على الإنترنت فقط

guard-local-title = هل تريد الاحتفاظ بنسخة من ملفاتك الموجودة على الإنترنت فقط؟
guard-local-body =
    { $count ->
        [zero] لا توجد ملفات في { $provider } محفوظة على الإنترنت فقط.
        [one] ملف واحد في { $provider } محفوظ على الإنترنت فقط. يمكنك الاحتفاظ بنسخة منه على الحاسوب المحمول الجديد أيضًا.
        [two] ملفان في { $provider } محفوظان على الإنترنت فقط. يمكنك الاحتفاظ بنسخة منهما على الحاسوب المحمول الجديد أيضًا.
        [few] { $count } ملفات في { $provider } محفوظة على الإنترنت فقط. يمكنك الاحتفاظ بنسخة منها على الحاسوب المحمول الجديد أيضًا.
        [many] { $count } ملفًا في { $provider } محفوظة على الإنترنت فقط. يمكنك الاحتفاظ بنسخة منها على الحاسوب المحمول الجديد أيضًا.
       *[other] { $count } ملف في { $provider } محفوظة على الإنترنت فقط. يمكنك الاحتفاظ بنسخة منها على الحاسوب المحمول الجديد أيضًا.
    }
guard-local-download = للتنزيل على الحاسوب المحمول القديم
guard-local-old-space = المساحة على الحاسوب المحمول القديم
guard-local-how = ينزّل PCTwin بضعة ملفات في كل مرة على الحاسوب المحمول القديم، ثم يرسلها، ثم يعيدها إلى الإنترنت فقط هناك.
guard-local-paused-metered = متوقف مؤقتًا: قد يحاسبك هذا الاتصال على البيانات. سيتابع من تلقاء نفسه عند توفر اتصال مجاني، أو الآن إن سمحت.
guard-local-paused-no-space = متوقف مؤقتًا: لم تعد هناك مساحة على الحاسوب المحمول القديم للملفات التالية. يبقى الباقي آمنًا في { $provider }.
guard-local-done = تم. النسخ موجودة على الحاسوب المحمول الجديد.
guard-local-keep = الاحتفاظ بنسخة
guard-local-leave = تركها في { $provider }
guard-local-allow-metered = السماح على هذا الاتصال

## M07 من يذهب إلى أين

guard-owners-title = من يذهب إلى أين
guard-owners-body = تذهب أشياء كل شخص إلى حسابه الخاص على هذا الحاسوب المحمول. تطابق الأسماء وحده لا يحسم ذلك أبدًا.
guard-owners-from = من { $laptop }
guard-owners-to-account = إلى الحساب { $account } على هذا الحاسوب المحمول
guard-owners-new-account = إلى حساب جديد على هذا الحاسوب المحمول
guard-owners-own-folder = مجلداتهم الخاصة "من { $laptop }"، لأن الأسماء متعارضة
guard-owners-usual-folders = مجلداتهم المعتادة
guard-owners-approve = موافقة

## X12 ما لم يكتمل

guard-report-title-partial = لم يُنقل بعض الأشياء
guard-report-title-done = تم نقل كل شيء في هذا الجزء
guard-report-body-partial = كل ما يلي لا يزال على الحاسوب المحمول القديم. أعد محاولة كل واحد عندما تكون مستعدًا. ما تم نقله يبقى.
guard-report-body-done = لا يوجد شيء متبقٍ لإعادة المحاولة.
guard-line-app = التطبيق { $name }
guard-line-items =
    { $count ->
        [zero] لا عناصر
        [one] عنصر واحد
        [two] عنصران
        [few] { $count } عناصر
        [many] { $count } عنصرًا
       *[other] { $count } عنصر
    }
guard-line-settings =
    { $count ->
        [zero] لا إعدادات
        [one] إعداد واحد
        [two] إعدادان
        [few] { $count } إعدادات
        [many] { $count } إعدادًا
       *[other] { $count } إعداد
    }
guard-line-why-failed = من { $laptop }: لم يكتمل.
guard-line-why-laptop-not-connected = من { $laptop }: ذلك الحاسوب المحمول غير متصل. وصّله بشبكة Wi-Fi نفسها وأبقه قيد التشغيل.
guard-line-why-waits-for-another = من { $laptop }: ينتظر خطوة أخرى لم تكتمل. أصلح تلك أولًا.
guard-line-why-not-started = من { $laptop }: توقفت عملية النقل قبل أن تبدأ.

## M25 وH10 موافقة النظام، ونقطة الاستعادة

guard-permission-title-explain = يحتاج PCTwin إلى موافقة واحدة من نظامك
guard-permission-title-waiting-for-system = أجب عن سؤال نظامك
guard-permission-title-denied = لم توافق
guard-permission-title-closed-without-answer = أُغلق السؤال دون إجابة
guard-permission-title-given = تمت الموافقة لعملية النقل هذه
guard-permission-title-ended-by-sleep = انتهت الموافقة عندما دخل الحاسوب المحمول في وضع السكون
guard-permission-title-ended-other-session = انتهت الموافقة عندما سجّل شخص آخر الدخول
guard-permission-body-explain = يسأل نظامك مرة واحدة لعملية النقل كلها. لا تستخدمها إلا الخطوات المذكورة أدناه، وتنتهي بانتهاء عملية النقل.
guard-permission-body-waiting-for-system = نافذة نظامك نفسه مفتوحة. اكتب كلمة المرور هناك إن طلبها.
guard-permission-body-denied = لا تُستبعد إلا الخطوات التي كانت تحتاج إليها. كل ما عداها يستمر.
guard-permission-body-closed-without-answer = إغلاق النافذة لا يُعدّ موافقة، لذلك لم يتم أي شيء يحتاج إليها. اطلبها مرة أخرى، أو تابع بدونها.
guard-permission-body-given = يستخدمها PCTwin للخطوات المذكورة أدناه فقط، وتنتهي بانتهاء عملية النقل هذه.
guard-permission-body-ended-by-sleep = لم يتم بعد ذلك أي شيء يحتاج إليها. اطلبها مرة أخرى لتتابع.
guard-permission-body-ended-other-session = لم يتم بعد ذلك أي شيء يحتاج إليها. اطلبها مرة أخرى عندما تعود إلى حسابك.
guard-permission-never-password = لا يطلب PCTwin كلمة مرورك أبدًا ولا يراها أبدًا. نظامك وحده هو من يراها.
guard-permission-not-now = تابع بدونها
guard-permission-ask = المتابعة إلى نظامك
guard-permission-ask-again = اطلبها مرة أخرى
guard-restore-title = نقطة استعادة في Windows
guard-restore-made = أُنشئت في { $time }.
guard-restore-earlier-exists = لدى Windows نقطة سابقة من { $time }، لذلك لم ينشئ أخرى.
guard-restore-not-chosen = لم تُنشأ: اخترت ألا تُنشأ.
guard-restore-not-made-not-windows = لم تُنشأ: نقاط الاستعادة موجودة في Windows فقط.
guard-restore-not-made-protection-off = لم تُنشأ: System Protection متوقفة لمحرك الأقراص هذا. يمكنك تشغيلها في System Properties.
guard-restore-not-made-needs-admin = لم تُنشأ: تحتاج إلى موافقة نظامك.
guard-restore-not-made-unconfirmed = لم تُنشأ: قال Windows نعم، لكن لا يمكن العثور على أي نقطة استعادة.
guard-restore-not-made-failed = لم تُنشأ: لم يستطع Windows إنشاء واحدة.

## T01 إلى T04 الإنقاذ من حاسوب محمول لا يعمل

guard-rescue-title-intro = انسخ الملفات من حاسوب محمول لا يبدأ التشغيل
guard-rescue-title-pick-drive = اختر محرك الأقراص المراد إنقاذه
guard-rescue-title-locked = محرك الأقراص هذا مقفل
guard-rescue-title-no-system = هذا ليس محرك أقراص حاسوب محمول
guard-rescue-title-salvage = هذا ما أمكن قراءته
guard-rescue-body-intro = أخرج محرك الأقراص من الحاسوب المحمول القديم ووصّله بهذا الحاسوب بمحوّل أو علبة لمحركات الأقراص.
guard-rescue-body-pick-drive = اختر محرك أقراص الحاسوب المحمول القديم. لا يمكن أن يكون محرك الأقراص الذي تنسخ إليه هو نفسه الذي تنقذه.
guard-rescue-body-locked = افتح قفله أولًا في نظامك. ثم تحقق مرة أخرى.
guard-rescue-body-no-system = لا يحتوي محرك الأقراص هذا على نظام حاسوب محمول ولا على مجلدات رئيسية. إن كان محرك USB عاديًا، فاستخدمه على الحاسوب المحمول الجديد بدلًا من ذلك.
guard-rescue-body-salvage = راجع القائمة، ثم استخدم ما أمكن قراءته.
guard-rescue-can = ما يستطيع الإنقاذ فعله
guard-rescue-can-body = قراءة ملفاتك، والأشخاص والإعدادات الموجودة على محرك الأقراص، من محرك أقراص ما زال يعمل.
guard-rescue-cannot = ما لا يستطيع فعله
guard-rescue-cannot-body = إصلاح محرك الأقراص، أو قراءة الأجزاء التالفة، أو فتح محرك أقراص ليس لديك مفتاحه.
guard-rescue-read-only-title = محرك الأقراص يُقرأ فقط
guard-rescue-read-only-promise = يقرأه PCTwin فقط. لا يتغيّر شيء عليه ولا يُصلَح ولا يُهيَّأ.
guard-rescue-system-read-only = فتحه نظامك للقراءة فقط، لذلك لا يمكن أن يتغيّر شيء عليه.
guard-rescue-pctwin-reads-only = قرأه PCTwin فقط. لكن نظامك كان قد فتحه للتعديل، لذلك كان بإمكان تطبيقات أخرى أن تغيّره.
guard-rescue-refused-target = محرك الأقراص هذا هو المكان الذي توضع فيه الأشياء، لذلك لا يمكن إنقاذه أيضًا. اختر محرك أقراص آخر.
guard-rescue-drive-detail = { $size }، { $fs }
guard-rescue-drive-is-target = تذهب أشياء عملية النقل إلى محرك الأقراص هذا.
guard-rescue-drive-cannot = لا يمكن إنقاذه
guard-rescue-choose = إنقاذ محرك الأقراص هذا
guard-rescue-bit-locker-step-1 = افتح محرك الأقراص في File Explorer.
guard-rescue-bit-locker-step-2 = اكتب مفتاح الاسترداد الخاص به عندما يطلبه Windows. غالبًا ما يكون محفوظًا في حساب Microsoft الخاص بالمالك، على aka.ms/myrecoverykey.
guard-rescue-bit-locker-step-3 = عد إلى هنا واختر التحقق مرة أخرى.
guard-rescue-file-vault-step-1 = افتح قفله على جهاز Mac بكلمة مرور تسجيل الدخول الخاصة بالمالك أو بمفتاح الاسترداد.
guard-rescue-file-vault-step-2 = عد إلى هنا واختر التحقق مرة أخرى.
guard-rescue-apple-chip-step-1 = يُبقي جهاز Mac هذا محرك أقراصه مقفلًا عليه وحده، لذلك لا يفيد إخراج محرك الأقراص.
guard-rescue-apple-chip-step-2 = إن كان جهاز Mac ما زال يبدأ في Recovery، فاستخدم Share Disk مع كابل. وإن لم يبدأ إطلاقًا، فلا يستطيع المساعدة إلا مركز إصلاح Apple.
guard-rescue-never-key = لا يطلب PCTwin المفتاح أبدًا ولا يحتفظ به أبدًا.
guard-rescue-readable =
    { $count ->
        [zero] لا يمكن قراءة أي عنصر.
        [one] يمكن قراءة عنصر واحد.
        [two] يمكن قراءة عنصرين.
        [few] يمكن قراءة { $n } عناصر.
        [many] يمكن قراءة { $n } عنصرًا.
       *[other] يمكن قراءة { $n } عنصر.
    }
guard-rescue-unreadable =
    { $count ->
        [zero] لا عناصر تعذّرت قراءتها
        [one] تعذّرت قراءة عنصر واحد
        [two] تعذّرت قراءة عنصرين
        [few] تعذّرت قراءة { $count } عناصر
        [many] تعذّرت قراءة { $count } عنصرًا
       *[other] تعذّرت قراءة { $count } عنصر
    }
guard-rescue-look = البحث عن محرك الأقراص
guard-rescue-check-again = التحقق مرة أخرى
guard-rescue-use = استخدام ما أمكن قراءته

## R05 وR06 التراجع

guard-undo-preview-title = ما الذي سيفعله التراجع
guard-undo-preview-body = نظر PCTwin في كل تغيير قبل لحظات. يتم الاحتفاظ بكل ما غيّرته بعد عملية النقل. لا يحدث شيء حتى تختار التراجع.
guard-undo-preview-which = هذا يتراجع عن عملية النقل التي انتهت في { DATETIME($when, year: "numeric", month: "long", day: "numeric", hour: "numeric", minute: "2-digit") }. لا تتغير عمليات النقل الأخرى.
guard-undo-preview-which-earlier = هذا يتراجع عن عملية النقل التي تمت بإصدار سابق من PCTwin. لا تتغير عمليات النقل الأخرى.
guard-undo-preview-admin = لإزالة التطبيقات، سيطلب هذا الحاسوب موافقة أحد المسؤولين.
guard-undo-plan-put-back = يُعاد كما كان
guard-undo-plan-remove-app = يزيل PCTwin التطبيق. لا يلمس PCTwin الملفات التي أنشأتها.
guard-undo-plan-keep-changed = محفوظ: لأنك غيّرته
guard-undo-plan-elsewhere = يحتاج إليك: تم في مكان آخر
guard-undo-plan-leave-alone = يُترك كما هو: كان موجودًا هنا من قبل
guard-undo-keep = إبقاء كل شيء كما هو
guard-undo-run = تراجع
guard-undo-running-title = جارٍ التراجع عن التغييرات
guard-undo-done-title = ما الذي فعله التراجع
guard-undo-result-restored = تمت الاستعادة
guard-undo-result-removed = تمت الإزالة
guard-undo-result-as-before = كان كما هو من قبل
guard-undo-result-kept-changed = محفوظ لأنك غيّرته
guard-undo-result-kept-other = محفوظ، انظر مزيد من التفاصيل
guard-undo-result-kept-at = محفوظ باسم { $name }
guard-undo-result-done-elsewhere = تم في مكان آخر: لا تستطيع التراجع عنه إلا خدمته أو مطوّره
guard-undo-result-not-done-yet = لم يتم بعد. سيحاول التراجع مرة أخرى في المرة القادمة.

## The full move: apps, settings and app data (28 October 2026)

fullmove-plan-title = التطبيقات والإعدادات وبيانات التطبيقات
fullmove-plan-body = اطّلع PCTwin على التطبيقات والإعدادات وبيانات التطبيقات في حاسوبك القديم. هذا ما سيفعله على هذا الحاسوب بعد ملفاتك. لا يتغيّر شيء حتى تبدأ النقل.
fullmove-group-install = تطبيقات سيثبّتها PCTwin: { $count }
fullmove-group-settings = إعدادات تطبيقات سينسخها PCTwin: { $count }
fullmove-group-needs = أشياء ستفعلها بنفسك: { $count }
fullmove-group-here = تطبيقات موجودة على هذا الحاسوب: { $count }
fullmove-settings-count = الإعدادات: { $count }.
fullmove-settings-merge = تُضاف. لا يُستبدل شيء على هذا الحاسوب.
fullmove-settings-replace = تُستبدل بعض الإعدادات على هذا الحاسوب. يمكن للتراجع إعادتها.
fullmove-app-install = يثبّته PCTwin من مطوّره ويتحقق منه.
fullmove-app-here = إنه موجود على هذا الحاسوب، لذلك يتركه PCTwin كما هو.
fullmove-step-copy = نُسخ، مع إمكانية التراجع.
fullmove-step-export-first = على حاسوبك القديم، استخدم التصدير الخاص بالتطبيق. ثم استورده في التطبيق على هذا الحاسوب.
fullmove-step-sign-in = سجّل الدخول إليه مرة أخرى على هذا الحاسوب.
fullmove-step-relink = أعد إعداده على هذا الحاسوب، أو اربطه مرة أخرى من التطبيق.
fullmove-step-not-yet = لا ينقل PCTwin بيانات هذا التطبيق بعد. ما زالت آمنة على حاسوبك القديم.
fullmove-step-look-at-it = لم يتمكن PCTwin من معرفة ما هذا. افحصه على حاسوبك القديم.
fullmove-group-update =
    { $count ->
        [zero] توجد إصدارات أقدم على هذا الحاسوب
        [one] يوجد إصدار أقدم على هذا الحاسوب
        [two] توجد إصدارات أقدم على هذا الحاسوب
        [few] توجد إصدارات أقدم على هذا الحاسوب
        [many] توجد إصدارات أقدم على هذا الحاسوب
       *[other] توجد إصدارات أقدم على هذا الحاسوب
    }
fullmove-update-it = تحديث { $name }
fullmove-update-hint = يثبّت PCTwin الإصدار الأحدث، وتبقى إعداداتك كما هي.
fullmove-special-not-yet = بيانات التطبيقات مثل البريد والمتصفحات والألعاب
fullmove-left-by-person = التطبيقات والإعدادات وبيانات التطبيقات: بقيت على حاسوبك القديم كما اخترت
fullmove-special-finish = لم ينقل PCTwin هذا. ما زال آمنًا على حاسوبك القديم.
fullmove-more = و{ $count } أخرى.
fullmove-not-moved-title = إعدادات يتركها PCTwin على الحاسوب القديم
fullmove-not-moved-private = إعدادات خاصة وأمنية، وتسجيلات دخول، لا تُنسخ أبدًا: { $count }
fullmove-not-moved-could-run = إعدادات قد تشغّل أو تفتح شيئًا، محتجزة للأمان: { $count }
fullmove-not-moved-old-laptop-only = إعدادات لا تناسب إلا الحاسوب القديم: { $count }
fullmove-not-moved-system-own = إعدادات النظام نفسه التي لا ينقلها PCTwin بعد: { $count }
fullmove-not-moved-not-read = إعدادات وُجدت ولم تُقرأ: { $count }
fullmove-special-not-moved = ملفات تطبيقات تُترك، مثل ذاكرات التخزين المؤقت: { $count }
fullmove-prompt = سيطلب هذا الحاسوب إذن المسؤول مرة واحدة.
fullmove-prompt-restore = سيطلب هذا الحاسوب إذن المسؤول مرة واحدة، وينشئ نقطة استعادة قبل أول تغيير.
fullmove-rebuilding-title = جارٍ إعداد تطبيقاتك وإعداداتك
fullmove-rebuilding-body = ملفاتك في مكانها. يُعدّ PCTwin الآن الباقي. أبقِ هذا الحاسوب يعمل.
fullmove-rebuilding-permission = جارٍ طلب الإذن من هذا الحاسوب
fullmove-rebuilding-apps = التطبيقات: { $done } من { $total }
fullmove-rebuilding-settings = الإعدادات: { $done } من { $total }
fullmove-outcome-title = التطبيقات والإعدادات وبيانات التطبيقات
fullmove-outcome-undo = تراجع عن إعدادات التطبيقات
fullmove-outcome-stopped = أوقف PCTwin تغيير الإعدادات في منتصف الطريق، لأنه لم يتمكن من تدوين تغييره التالي. لم يتغيّر شيء بعد تلك النقطة.
fullmove-restore-made = أُنشئت نقطة استعادة قبل التغييرات.
fullmove-restore-earlier-exists = لم تُنشأ نقطة استعادة جديدة، لأن هذا الحاسوب كانت لديه واحدة من اليوم.
fullmove-restore-not-made = لم تُنشأ نقطة استعادة. ما زال تراجع PCTwin يعمل.
fullmove-restore-not-windows = لا توجد نقاط استعادة على هذا الحاسوب. ما زال تراجع PCTwin يعمل.
guard-undo-remove-yourself = أزله بنفسك إن أردت
guard-undo-plan-remove-yourself = يمكنك إزالته من إعدادات حاسوبك
guard-undo-result-remove-yourself = متروك لك لإزالته من إعدادات حاسوبك
guard-undo-plan-keep-other-version = محفوظ: تغيّر منذ عملية النقل
guard-undo-plan-already-gone = تمت إزالته من قبل
guard-undo-result-kept-other-version = محفوظ لأنه تغيّر منذ عملية النقل
guard-undo-result-removed-restart = تمت الإزالة. أعد تشغيل حاسوبك لإكمالها.
guard-undo-result-not-removed = لم تتم الإزالة. يمكنك إزالته من إعدادات حاسوبك.
guard-undo-close = تم
move-wait-appdata-title = جارٍ الاطلاع على بيانات تطبيقاتك
move-wait-appdata-body = عُثر على ملفاتك. ما زال PCTwin يطّلع على البيانات التي تحفظها تطبيقاتك، مثل البريد والمتصفحات. يستغرق ذلك عشر دقائق على الأكثر. لا يتغيّر شيء على هذا الحاسوب.
fullmove-special-not-checked = بيانات تطبيقات لم يتمكن PCTwin من فحصها بالكامل
fullmove-group-appdata = بيانات تطبيقات سينسخها PCTwin: { $count }
fullmove-group-secrets = ملفات سرية يمكنك اختيار إحضارها: { $count }
fullmove-secrets-body = تحتوي على كلمات مرور أو مفاتيح. لا يُحضر PCTwin أيًا منها إلا إذا حددته بنفسك. إن لم تحدده، يبقى على حاسوبك القديم.
fullmove-secret-bring = إحضار { $name }
fullmove-secret-hint = أنت وحدك يمكنك فتحه على هذا الحاسوب. يمكن للتراجع إزالته مرة أخرى.
fullmove-appdata-title = جارٍ نسخ بيانات تطبيقاتك
fullmove-appdata-progress = جارٍ نسخ بيانات التطبيقات: { $done } من { $total }
fullmove-appdata-waiting = بانتظار الحاسوب القديم: أغلق { $apps } هناك.
fullmove-close-title = أغلق هذه التطبيقات لإحضار بياناتها
fullmove-close-body = هذه التطبيقات ما زالت مفتوحة على هذا الحاسوب، لذلك لا يستطيع PCTwin نسخ بياناتها بأمان. أغلقها، ثم اختر حاول مرة أخرى. إن تابعت بدونها، تبقى بياناتها هنا.
fullmove-close-again = حاول مرة أخرى
fullmove-close-without = تابع بدونها
fullmove-extras-refused = تعذّرت قراءة إعدادات وبيانات تطبيقات الحاسوب القديم هنا، لذلك لم يُنقل شيء منها. ما زالت على حاسوبك القديم.
fullmove-app-from = يثبّت PCTwin حزمة winget ‏{ $id } من مطوّرها، بعد التحقق من أنها هذا التطبيق ومن المطوّر نفسه.
fullmove-here-older = يوجد على هذا الحاسوب إصدار أقدم ({ $version }). يتركه PCTwin كما هو.
fullmove-step-sites = تسجيلات الدخول المحفوظة إلى المواقع تبقى مرتبطة بالحاسوب القديم. سجّل الدخول إلى هذه المواقع مرة أخرى على هذا الحاسوب.
fullmove-app-rosetta = صُمّم لأجهزة Mac القديمة بمعالج Intel. يعمل على هذا الـMac عبر Rosetta، الذي ستوقفه Apple، لذا قد يتوقف عن العمل بعد تحديث لاحق لنظام macOS. ابحث عن إصدار أحدث من الشركة المصنّعة.
fullmove-links = اختصارات إلى أماكن أخرى بين ملفاتك، لا يتبعها PCTwin ولا ينقلها: { $count }
fullmove-note-sign-in = افتحه وسجّل الدخول إلى حسابك.
fullmove-note-licence-key = افتحه وأدخل مفتاح الترخيص، أو سجّل الدخول إلى الحساب الذي يحمل ترخيصك.
fullmove-note-registration = كان حاسوبك القديم يُظهره مسجّلًا باسم { $who }.
fullmove-note-product-id = كان معرّف المنتج على حاسوبك القديم { $id }.
fullmove-note-extras = جاءت معه برامج أخرى: { $apps }. أزلها من إعدادات حاسوبك إن لم تكن تريدها.
fullmove-note-rosetta = صُمّم لأجهزة Mac بمعالج Intel، لذا يحتاج إلى Rosetta. إذا عرض هذا الـMac تثبيت Rosetta عند فتح التطبيق، فاختر تثبيت.
fullmove-data-lost = انقطع الاتصال قبل وصول هذه البيانات. ما زالت آمنة على حاسوبك القديم. ابدأ نقلًا جديدًا لإحضارها.
fullmove-data-did-not-arrive = لم تصل هذه البيانات. ما زالت آمنة على حاسوبك القديم. ابدأ نقلًا جديدًا لإحضارها.
fullmove-data-app-open = كان التطبيق مفتوحًا على حاسوبك القديم، لذلك لم تُنسخ بياناته. أغلق التطبيق هناك، ثم ابدأ نقلًا جديدًا لإحضارها.
fullmove-data-changing = ظلت هذه البيانات تتغير على حاسوبك القديم أثناء قراءة PCTwin لها. أغلق التطبيق هناك، ثم ابدأ نقلًا جديدًا لإحضارها.
fullmove-data-unreadable = لم يتمكن PCTwin من قراءة هذه البيانات على حاسوبك القديم. ما زالت هناك.
fullmove-data-not-read = توقف النقل قبل قراءة هذه البيانات. ما زالت آمنة على حاسوبك القديم.
fullmove-data-not-whole = لم تصل كل هذه البيانات، لذلك لم يضع PCTwin أيًّا منها في مكانه. ما زالت آمنة على حاسوبك القديم.
fullmove-data-not-landed = وصلت هذه البيانات، لكن PCTwin لم يتمكن من وضعها في مكانها على هذا الحاسوب. ما زالت آمنة على حاسوبك القديم.
fullmove-data-beside = كان للتطبيق مجلده الخاص على هذا الحاسوب، لذلك وضع PCTwin نسخة الحاسوب القديم بجانبه، في { $folder }. ما زال التطبيق يستخدم مجلده الخاص. لاستخدام النسخة القديمة بدلًا منه، أغلق التطبيق وبدّل المجلدين.
fullmove-data-placed = البيانات في { $folder }، لكن التطبيق يحتاج إلى خطوة قبل أن يستخدمها. افتح التطبيق وتحقق منها.
fullmove-outcome-here-title = تطبيقات موجودة على هذا الحاسوب: { $count }
fullmove-outcome-here = موجود على هذا الحاسوب، وتُرك كما هو.
fullmove-outcome-here-older = يوجد على هذا الحاسوب إصدار أقدم ({ $version }). تركه PCTwin كما هو. حدّثه من صانعه إن أردت الإصدار الأحدث.
fullmove-outcome-here-badge = تُرك كما هو
fullmove-outcome-fresh = التطبيقات التي ثبّتها PCTwin تبدأ من جديد. افتح كل واحد منها وسجّل الدخول إن طلب ذلك.
progress-may-sleep = قد يدخل هذا الحاسوب في وضع السكون أثناء النقل. أبقه موصولًا بالكهرباء ومفتوحًا.

## The old laptop chooses what moves (decided 28 October 2026)
choose-title = ماذا تريد أن تنقل؟
choose-body = اختر هنا، على الحاسوب الذي فيه أغراضك. بعدها يعرض حاسوبك الجديد ما سيصل ويطلب منك قبوله.
choose-mode = ما الذي يُنقل
choose-everything = كل شيء
choose-except = كل شيء ما عدا
choose-only = فقط ما أختاره
choose-except-hint = أزل العلامة عمّا تتركه.
choose-open = فتح { $name }
choose-close = إغلاق { $name }
choose-total =
    { $files ->
        [zero] { $files } من الملفات
        [one] ملف واحد
        [two] { $files } من الملفات
        [few] { $files } من الملفات
        [many] { $files } من الملفات
       *[other] { $files } من الملفات
    } · المختار: { $size }
choose-only-hint = ضع علامة على ما تأخذه.
choose-apps = التطبيقات والإعدادات وبيانات التطبيقات
choose-apps-count =
    { $count ->
        [zero] { $count } من التطبيقات مع إعداداتها
        [one] تطبيق واحد مع إعداداته
        [two] { $count } من التطبيقات مع إعداداتها
        [few] { $count } من التطبيقات مع إعداداتها
        [many] { $count } من التطبيقات مع إعداداتها
       *[other] { $count } من التطبيقات مع إعداداتها
    }
choose-group-size =
    { $files ->
        [zero] { $files } من الملفات
        [one] ملف واحد
        [two] { $files } من الملفات
        [few] { $files } من الملفات
        [many] { $files } من الملفات
       *[other] { $files } من الملفات
    } · { $size }
choose-left-title = يبقى هنا في كل الأحوال
choose-left-badge = يبقى
choose-left-caches = ذاكرة التخزين المؤقت والملفات المؤقتة
choose-left-chosen-by-user = ما اخترت تركه
choose-left-work-managed = ملفات عملك أو مدرستك
choose-left-device-bound = أشياء مرتبطة بهذا الحاسوب، مثل مفاتيح المرور
choose-left-unreadable = ملفات لم يستطع PCTwin قراءتها
choose-left-cloud-only = ملفات محفوظة على الإنترنت فقط
choose-left-system = أجزاء من Windows نفسه
choose-left-too-many-files = مجلدات أكبر من أن تُنقل مرة واحدة
choose-left-rebuildable = أشياء يستطيع حاسوبك الجديد إنشاءها من جديد، مثل الحزم المنزّلة
choose-nothing = اختر شيئًا واحدًا على الأقل لنقله.
choose-continue = متابعة

## The new laptop accepts what the old laptop chose (decided 28 October 2026)
accept-title = هل تقبل النقل من { $from }؟
accept-body = تم اختيار هذا على حاسوبك القديم. اقبله، وينقله PCTwin إلى هنا.
accept-files =
    { $count ->
        [zero] { $count } من الملفات
        [one] ملف واحد
        [two] { $count } من الملفات
        [few] { $count } من الملفات
        [many] { $count } من الملفات
       *[other] { $count } من الملفات
    }
accept-apps =
    { $count ->
        [zero] { $count } من التطبيقات للتثبيت
        [one] تطبيق واحد للتثبيت
        [two] { $count } من التطبيقات للتثبيت
        [few] { $count } من التطبيقات للتثبيت
        [many] { $count } من التطبيقات للتثبيت
       *[other] { $count } من التطبيقات للتثبيت
    }
accept-settings = الإعدادات وبيانات التطبيقات
accept-fits = يتسع له هذا الحاسوب.
accept-needs = هناك ما يحتاج إليك أولًا. اطّلع على التفاصيل.
accept-not-now = ليس الآن
accept-details = عرض التفاصيل
accept-accept = قبول

# 2.5 special categories
special-category-email = البريد
special-category-browser = المتصفحات
special-category-game = الألعاب
special-category-ai-tool = أدوات الذكاء الاصطناعي ومحررات الشيفرة
special-category-secret = المفاتيح وكلمات المرور
special-category-other = بيانات تطبيقات أخرى
special-note-search-rebuilds = يعيد تطبيق البريد بناء البحث في أول مرة يُفتح فيها. قد يكون البحث بطيئًا لبعض الوقت.
special-note-server-resync = يعود بريدك من خادم البريد عندما تسجّل الدخول. يحتفظ PCTwin بالنسخة القديمة فقط تحسّبًا لشيء لم يصل إلى الخادم.
special-note-own-sync = أسهل طريقة: شغّل المزامنة الخاصة بهذا المتصفح على حاسوبك الجديد. فهي تجلب الإشارات المرجعية والسجل وعلامات التبويب المفتوحة والإضافات.
special-note-passwords-guided = لا تنتقل كلمات المرور المحفوظة إلا عبر التصدير والاستيراد الخاصين بالمتصفح نفسه، أو عبر مزامنته. الملف المُصدَّر غير محمي، لذا يبقيه PCTwin خاصًا ويحذفه بعد الاستيراد.
special-note-passkeys-guided = مفاتيح المرور المحفوظة في حساب تعود عندما تسجّل الدخول إليه. أما المحفوظة على هذا الحاسوب فقط فيجب إنشاؤها من جديد في كل موقع. افعل ذلك قبل مسح الحاسوب القديم.
special-note-cloud-saves = قد يحتفظ مشغّل اللعبة بحفظاتك على الإنترنت أيضًا. إذا سألك أيّ الحفظات تبقي، فانظر إلى التواريخ أولًا.
special-note-models-checked = تُفحص نماذج الذكاء الاصطناعي ملفًا ملفًا عند وصولها. لا يشغّلها PCTwin أبدًا.
special-note-tool-settings-review = قد تشغّل هذه الإعدادات برامج أو تتصل بالإنترنت. راجعها قبل استخدامها على الحاسوب الجديد.
special-note-secret-tick-each = لا ينتقل هذا إلا إذا وضعت علامة عليه. ولا يظهر أبدًا في أي تقرير.
forgotten-title = أشياء ينساها الناس كثيرًا
forgotten-kind-fonts = خطوطك الخاصة
forgotten-kind-mail-signatures = تواقيع البريد
forgotten-kind-dictionaries = كلماتك الخاصة في القاموس
forgotten-kind-office-templates = قوالب Office
forgotten-kind-mail-archives = بريد محفوظ على هذا الحاسوب
forgotten-kind-game-saves = حفظات الألعاب
forgotten-kind-game-library = ألعاب ثبّتها مشغّل ألعاب
forgotten-moving = ينتقل هذا مع بقية أغراضك.
forgotten-not-moving = هذا ليس ضمن النقل. يمكنك نسخه بنفسك.
forgotten-step-install = بعد النقل، افتح كل خط على الحاسوب الجديد واختر تثبيت.
forgotten-step-locate-verify = ثبّت مشغّل الألعاب على الحاسوب الجديد وسجّل الدخول. يمكنه تثبيت ألعابك من جديد. إذا نسخت هذا المجلد، فاستخدم خيار المشغّل للعثور على الألعاب المثبّتة، ودعه يفحص الملفات.
forgotten-not-whole = توقّف PCTwin عن العدّ هنا، لذا قد يكون هناك المزيد.
forgotten-printers = طابعات يجب إضافتها من جديد
forgotten-printer-step = أضف هذه الطابعة من جديد على حاسوبك الجديد.
forgotten-printer-network = مشتركة من حاسوب آخر
forgotten-not-checked = لم يتمكّن PCTwin من النظر هنا.
forgotten-files =
    { $count ->
        [zero] { $count } من الملفات
        [one] ملف واحد
        [two] { $count } من الملفات
        [few] { $count } من الملفات
        [many] { $count } من الملفات
       *[other] { $count } من الملفات
    }
forgotten-online-only =
    { $count ->
        [zero] { $count } من الملفات محفوظة على الإنترنت فقط، ولم تُنزَّل
        [one] ملف واحد محفوظ على الإنترنت فقط، ولم يُنزَّل
        [two] { $count } من الملفات محفوظة على الإنترنت فقط، ولم تُنزَّل
        [few] { $count } من الملفات محفوظة على الإنترنت فقط، ولم تُنزَّل
        [many] { $count } من الملفات محفوظة على الإنترنت فقط، ولم تُنزَّل
       *[other] { $count } من الملفات محفوظة على الإنترنت فقط، ولم تُنزَّل
    }

# 2.7 finishing a move
doctor-not-connected = هذا الحاسوب غير متصل بأي شبكة.
doctor-not-connected-fix = اتصل بشبكة Wi-Fi أو صِل كابل شبكة، ثم حاول مرة أخرى.
doctor-mac-local-network-denied = قد لا يسمح جهاز Mac هذا لـ PCTwin بالتواصل مع الأجهزة على شبكتك.
doctor-mac-local-network-denied-fix = افتح إعدادات النظام، ثم الخصوصية والأمن، ثم الشبكة المحلية، وفعّل PCTwin.
doctor-discovery-blocked = قد يكون شيء على هذا الحاسوب، مثل جدار حماية أو تطبيق أمان، يمنع الحاسوبين من إيجاد بعضهما.
doctor-discovery-blocked-fix = اسمح لـ PCTwin في جدار الحماية أو تطبيق الأمان، أو أوقفه مؤقتًا أثناء النقل.
doctor-windows-public-network = يعامل Windows الشبكة { $name } كشبكة عامة، وهذا يحجب الأجهزة الأخرى.
doctor-windows-public-network-fix = افتح الإعدادات، ثم الشبكة والإنترنت، واختر { $name }، واجعلها شبكة خاصة.
doctor-vpn-on = هناك VPN قيد التشغيل ({ $name }). قد يُخفي شبكة منزلك.
doctor-vpn-on-fix = أوقف VPN أثناء النقل، ثم حاول مرة أخرى.
doctor-old-laptop-not-pairing = قد لا يعرض الحاسوب القديم رمزه.
doctor-old-laptop-not-pairing-fix = على الحاسوب القديم، افتح PCTwin واختر أنه حاسوبك القديم.
doctor-different-networks = قد يكون الحاسوبان على شبكتين مختلفتين.
doctor-different-networks-fix = صِل الحاسوبين بشبكة Wi-Fi نفسها.
doctor-guest-network = قد تمنع هذه الشبكة الأجهزة من رؤية بعضها، كما يحدث غالبًا في شبكات Wi-Fi للضيوف والفنادق والمكاتب.
doctor-guest-network-fix = استخدم شبكة Wi-Fi في منزلك، أو نقطة الاتصال في هاتفك.
doctor-probe-interfaces = تعذّر على PCTwin عرض اتصالات الشبكة لهذا الحاسوب.
doctor-probe-hears-itself = تعذّر على PCTwin اختبار ما إذا كان يمكن العثور على هذا الحاسوب على الشبكة.
doctor-probe-local-network = تعذّر على PCTwin التحقق من إعداد الشبكة المحلية في Mac.
doctor-probe-network-profiles = تعذّر على PCTwin التحقق من كيفية تعامل Windows مع هذه الشبكة.
doctor-hotspot = ما زال لا يعمل؟ شغّل نقطة الاتصال في هاتفك وصِل الحاسوبين بها. يتم النقل مباشرة بين الحاسوبين، وليس عبر بيانات هاتفك.
windows-make-other-administrator = تأكد من أن الشخص الذي سيحتفظ بالحاسوب لديه حساب مسؤول عليه.
windows-sign-in-other-administrator = سجّل الخروج، ثم سجّل الدخول بحساب المسؤول الآخر. لا يمكن لأي حساب أن يحذف نفسه.
windows-open-other-users = افتح الإعدادات، ثم الحسابات، ثم المستخدمون الآخرون.
windows-remove-account = اختر حسابك، ثم إزالة.
windows-delete-account-and-data = اختر حذف الحساب والبيانات. تبقى حسابات الآخرين كما هي.
windows-sign-out-of-everything = أولًا سجّل الخروج من تطبيقاتك وحساباتك على الحاسوب القديم.
windows-open-recovery = افتح الإعدادات، ثم النظام، ثم الاسترداد.
windows-reset-this-pc = اختر إعادة تعيين الكمبيوتر.
windows-remove-everything = اختر إزالة كل شيء.
windows-clean-the-drive = غيّر الإعدادات ليتم تنظيف محرك الأقراص، لا تفريغه فقط. يستغرق ذلك بضع ساعات.
windows-wait-until-setup-screen = انتظر حتى يعرض الحاسوب شاشة الإعداد. عندها يكون جاهزًا للتسليم.
macos-make-other-administrator = تأكد من أن الشخص الذي سيحتفظ بجهاز Mac لديه حساب مسؤول عليه.
macos-sign-in-other-administrator = سجّل الخروج، ثم سجّل الدخول بحساب المسؤول الآخر. لا يمكن لأي حساب أن يحذف نفسه.
macos-open-users-and-groups = افتح إعدادات النظام، ثم المستخدمون والمجموعات.
macos-delete-user = اختر زر المعلومات بجانب حسابك، ثم حذف المستخدم.
macos-delete-home-folder = اختر حذف مجلد الحساب الرئيسي.
macos-sign-out-of-apple-account = أولًا سجّل الخروج من حساب Apple على جهاز Mac القديم، ليصبح متاحًا لمالكه التالي.
macos-open-transfer-or-reset = افتح إعدادات النظام، ثم عام، ثم النقل أو إعادة التعيين.
macos-erase-all-content-and-settings = اختر مسح كل المحتوى والإعدادات، واتبع الخطوات.
macos-older-mac-use-recovery = إذا لم يعرض جهاز Mac ذلك (أجهزة Mac الأقدم)، شغّله في استرداد macOS، وامسح القرص بأداة القرص، ثم أعد تثبيت macOS.
macos-wait-until-setup-screen = انتظر حتى يعرض جهاز Mac شاشة الإعداد. عندها يكون جاهزًا للتسليم.
linux-sign-in-other-administrator = سجّل الدخول بحساب مسؤول آخر.
linux-open-users = افتح الإعدادات، ثم المستخدمون.
linux-remove-user-and-files = اختر حسابك، ثم إزالة المستخدم، واختر حذف ملفاته.
linux-no-built-in-reset = لا يحتوي Linux على إعادة تعيين مدمجة تمسح محرك الأقراص.
linux-drive-maker-secure-erase = استخدم أداة المسح الآمن من الشركة المصنّعة لمحرك الأقراص، أو المسح الخاص بالحاسوب في إعدادات بدء التشغيل إن وُجد.
linux-reinstall-from-usb = ثم ثبّت نظامًا جديدًا من ذاكرة USB.
any-sign-in-other-administrator = سجّل الدخول بحساب مسؤول آخر على الحاسوب القديم.
any-use-system-account-settings = احذف حسابك من إعدادات الحسابات في النظام نفسه.
any-use-system-reset = استخدم إعادة التعيين الخاصة بالحاسوب التي تمسح محرك الأقراص. تشرح مساعدة الشركة المصنّعة الطريقة.
any-drive-maker-secure-erase = إن لم توجد، استخدم أداة المسح الآمن من الشركة المصنّعة لمحرك الأقراص.

# 2.9 deliverables
guard-undo-removes-if-package = يزيله PCTwin إذا ثُبّت من حزمة تثبيت؛ وإلا فأزله بنفسك
guard-undo-keeps-newer = التراجع يُبقي الإصدار الأحدث: لا يمكن إرجاع الإصدار الأقدم
compat-outcome-works = تم اختباره: يعمل
compat-outcome-works-with-steps = تم اختباره: يعمل مع بعض الخطوات منك
compat-outcome-partly = تم اختباره: بعض الأجزاء تعمل
compat-outcome-not-supported = تم اختباره: لا يستطيع PCTwin إعداده هنا
compat-outcome-not-tested = لم يُختبر بعد
compat-aspect-install = التثبيت
compat-aspect-data-import = نقل بياناته
compat-aspect-settings = إعداداته
compat-aspect-launch = فتحه
compat-aspect-undo = التراجع
compat-result-works = يعمل
compat-result-guided = يعمل بمساعدتك
compat-result-not-supported = لا يعمل
compat-result-not-applicable = غير مطلوب
compat-result-not-tested = لم يُختبر
compat-step-sign-in = سجّل الدخول
compat-step-licence = أدخل مفتاح الترخيص
compat-step-import-in-app = استورد بياناتك في التطبيق
compat-step-relink = وجّهه إلى ملفاتك مرة أخرى
compat-step-restart = أعد تشغيل الحاسوب
compat-how-lab = اختبره PCTwin على حواسيب نظيفة
compat-how-real-laptops = تم اختباره على حواسيب حقيقية
work-what-linked-files = الملفات المرتبطة
work-what-font = الخط
work-what-archive = الأرشيف يُفتح
work-what-database = قاعدة البيانات تُفتح
work-what-task = شيء تتحقق منه بنفسك
work-label-machine-checked = تحقق منه PCTwin
work-label-person-confirmed = تحققت منه أنت
work-label-still-needs-attention = لا يزال يحتاج إلى تحقق
work-state-not-run-yet = لم يُتحقق منه بعد
work-state-all-there = كل شيء موجود
work-state-missing = هناك شيء ناقص
work-state-damaged = غير مكتمل
work-state-not-there = غير موجود
work-state-waiting-for-you = في انتظارك
work-state-you-said-no = قلت إنه لم ينجح
work-state-outside-project = روابط خارج مجلده: لم تُفحص
work-state-could-not-read = تعذّرت قراءته
work-state-not-supported = لا يستطيع PCTwin فحص هذا النوع من الملفات بعد
work-state-partly-checked = لم يُفحص كل شيء
work-problem-not-chosen = لم يُختر أي ملف.
work-problem-too-many = فحوص كثيرة في وقت واحد. أزل بعضها أولًا.
work-problem-no-such-check = هذا الفحص لم يعد موجودًا.
work-problem-stale = تغيّرت القائمة. انظر إليها مرة أخرى.
work-problem-bad-name = اكتب اسمًا لا يزيد على 200 حرف.
work-problem-wrong-kind = هذه الإجابة لا تناسب هذا الفحص.
work-problem-failed = حدث خطأ ما. حاول مرة أخرى.
repair-waits-old-laptop = يحتاج إلى توصيل الحاسوب القديم
repair-waits-internet = يحتاج إلى الإنترنت
repair-waits-administrator = يحتاج إلى موافقة مسؤول
repair-waits-installer-tool = يحتاج إلى مثبّت التطبيقات في النظام
repair-waits-space = يحتاج إلى مساحة أكبر على هذا الحاسوب
repair-waits-close-app = يحتاج إلى إغلاق التطبيق
repair-problem-nothing-to-repair = لا شيء مما اخترته يحتاج إلى إصلاح.
repair-problem-stale = تغيّرت القائمة. انظر إليها مرة أخرى.
repair-problem-busy = PCTwin مشغول بشيء آخر. حاول عندما ينتهي.
repair-problem-failed = حدث خطأ ما. لم يتغيّر أي شيء آخر.
changed-why-app-updated = تم تحديث التطبيق بعد موافقتك
changed-why-app-removed = تمت إزالة التطبيق بعد موافقتك
changed-why-destination-changed = تغيّر المجلد الذي يذهب إليه
changed-why-project-edited = تم تغييره على الحاسوب المحمول القديم بعد موافقتك
changed-step-install = التثبيت
changed-step-settings = الإعدادات
changed-step-app-data = بيانات التطبيقات
changed-step-files = الملفات
safe-stop-busy-moving = نقل الملفات
safe-stop-busy-rebuilding = إعداد التطبيقات والإعدادات
safe-stop-busy-app-data = بيانات التطبيقات
safe-stop-busy-undoing = التراجع عن نقل
kit-part-move-record = سجل النقل الخاص بك
kit-part-results = نتيجة كل عنصر
kit-part-before-wipe-list = ما لا يزال موجودًا على الحاسوب القديم فقط
kit-part-file-data = ملفاتك
kit-part-setup-steps = خطوات إعداد التطبيقات من جديد
kit-part-recovery-tool = أداة لاستعادة الأشياء

# 2.8 screens
report-group-apps = التطبيقات
report-group-settings = الإعدادات
report-group-special = بيانات التطبيقات وأشياء أخرى
report-group-people = أشخاص آخرون
report-count-apps = { $count ->
        [zero] لا تطبيقات
        [one] تطبيق واحد
        [two] تطبيقان
        [few] { $count } تطبيقات
        [many] { $count } تطبيقًا
       *[other] { $count } تطبيق
    }
report-count-settings = { $count ->
        [zero] لا مجموعات إعدادات
        [one] مجموعة إعدادات واحدة
        [two] مجموعتا إعدادات
        [few] { $count } مجموعات إعدادات
        [many] { $count } مجموعة إعدادات
       *[other] { $count } مجموعة إعدادات
    }
report-count-special = { $count ->
        [zero] لا شيء
        [one] شيء واحد
        [two] شيئان
        [few] { $count } أشياء
        [many] { $count } شيئًا
       *[other] { $count } شيء
    }
report-count-people = { $count ->
        [zero] لا أحد
        [one] شخص واحد
        [two] شخصان
        [few] { $count } أشخاص
        [many] { $count } شخصًا
       *[other] { $count } شخص
    }
report-beyond-done = اكتمل كل شيء وتم التحقق منه
report-beyond-some = اكتمل { $done } من { $total } وتم التحقق منها
report-beyond-left = { $count ->
        [zero] لا شيء لم يُنقل
        [one] واحد لم يُنقل: تركته أنت
        [two] اثنان لم يُنقلا: تركتهما أنت
        [few] { $count } لم تُنقل: تركتها أنت
        [many] { $count } لم يُنقل: تركتها أنت
       *[other] { $count } لم يُنقل: تركتها أنت
    }
report-beyond-more = { $count ->
        [zero] لا شيء آخر.
        [one] واحد آخر لا يظهر هنا.
        [two] اثنان آخران لا يظهران هنا.
        [few] { $count } أخرى لا تظهر هنا.
        [many] { $count } آخر لا يظهر هنا.
       *[other] { $count } آخر لا يظهر هنا.
    }
report-here = { $count ->
        [zero] لم يكن أي تطبيق موجودًا على هذا الحاسوب.
        [one] كان تطبيق واحد موجودًا على هذا الحاسوب. تركه PCTwin كما هو.
        [two] كان تطبيقان موجودين على هذا الحاسوب. تركهما PCTwin كما هما.
        [few] كانت { $count } تطبيقات موجودة على هذا الحاسوب. تركها PCTwin كما هي.
        [many] كان { $count } تطبيقًا موجودًا على هذا الحاسوب. تركها PCTwin كما هي.
       *[other] كان { $count } تطبيق موجودًا على هذا الحاسوب. تركها PCTwin كما هي.
    }
progress-ready-title = جاهز للاستخدام الآن
progress-ready-body = { $count ->
        [zero] لا شيء جاهز بعد.
        [one] { $groups } نُسخ وتم التحقق منه. يمكنك فتحه على هذا الحاسوب بينما يصل الباقي.
        [two] { $groups } نُسخا وتم التحقق منهما. يمكنك فتحهما على هذا الحاسوب بينما يصل الباقي.
        [few] { $groups } نُسخت وتم التحقق منها. يمكنك فتحها على هذا الحاسوب بينما يصل الباقي.
        [many] { $groups } نُسخت وتم التحقق منها. يمكنك فتحها على هذا الحاسوب بينما يصل الباقي.
       *[other] { $groups } نُسخت وتم التحقق منها. يمكنك فتحها على هذا الحاسوب بينما يصل الباقي.
    }
finish-strip = أنهِ عملية النقل
finish-title = أنهِ عملية النقل
finish-loading = جارٍ عرض عملية النقل.
finish-retry = حاول مرة أخرى
finish-back-to-list = العودة إلى القائمة
finish-open = فتح
finish-open-name = فتح { $title }
finish-more-not-shown = عناصر أخرى غير معروضة: { $count }
finish-problem-no-move = لا توجد لدى PCTwin عملية نقل لإنهائها على هذا الحاسوب المحمول. لم يتغير شيء.
finish-problem-changed = ما يظهر قديم. انظر إليه مرة أخرى قبل أن تجيب. لم يتغير شيء.
finish-problem-not-allowed = لا يمكن القيام بذلك. لم يتغير شيء.
finish-problem-busy = هناك عملية نقل أو خطوة أخرى قيد التنفيذ. انتظر حتى تنتهي ثم حاول مرة أخرى. لم يتغير شيء.
finish-problem-failed = حدث خطأ على هذا الحاسوب المحمول. لم يتغير شيء. حاول مرة أخرى.
finish-headline-ready-for-selected-work = كل ما اخترته موجود على هذا الحاسوب المحمول
finish-headline-ready-for-selected-work-body = وجد PCTwin كل عنصر اخترته على هذا الحاسوب المحمول وتحقق منه بقدر ما يستطيع. هذا لا يضمن أن كل تطبيق سيعمل كما كان.
finish-headline-finished-with-tasks = اكتمل النقل، وبقيت بعض المهام
finish-headline-finished-with-tasks-body = بعض العناصر تحتاج إليك، أو لم يتم نسخها. وهي مدرجة في هذه الصفحة.
finish-headline-data-preserved = كل شيء هنا، لكن بعضه يحتاج إلى مفتاح لفتحه
finish-headline-data-preserved-body = كل شيء موجود على هذا الحاسوب المحمول. فتح بعضها يحتاج إلى مفتاح أو رمز استرداد، ولم يستطع PCTwin التحقق من ذلك هنا.
finish-headline-handover-unresolved = لم يتم تسليم الحاسوب المحمول القديم بعد
finish-headline-handover-unresolved-body = انتهى العمل على هذا الحاسوب المحمول. قائمة ما قبل ترك الحاسوب المحمول القديم ما زالت فيها أمور مفتوحة.
finish-headline-nothing-moved = لم يتم اختيار شيء للنقل
finish-headline-nothing-moved-body = لم تُختر أي ملفات أو تطبيقات أو إعدادات، لذلك لم يُنسخ شيء.
finish-tally-title = ما تم نقله
finish-tally-selected = المختار: { $count }
finish-tally-done = المنسوخ والمتحقق منه: { $count }
finish-tally-needs-action = يحتاج إليك: { $count }
finish-tally-failed = لم يُنسخ: { $count }
finish-tally-waiting = ما زال ينتظر: { $count }
finish-tally-not-reachable = الحاسوب المحمول القديم غير متاح: { $count }
finish-tally-no-result-yet = لا نتيجة بعد: { $count }
finish-tally-skipped = تجاوزته أنت: { $count }
finish-tally-left-out = المستبعد: { $count }
finish-tally-accepted-loss = تُرك عمدًا: { $count }
finish-others = عناصر أشخاص آخرين، تُحسب فقط ولا تُعرض أبدًا: { $count }
finish-after-title = تم التحقق على هذا الحاسوب المحمول الآن
finish-after-body = عدّ PCTwin هذه العناصر على الحاسوب المحمول القديم، ثم عدّها مرة أخرى على هذا الحاسوب المحمول.
finish-after-count = { $onNew } من { $atScan } موجودة على هذا الحاسوب المحمول
finish-after-all = كلها موجودة
finish-after-short = بعضها ناقص
finish-group-photos = الصور
finish-group-videos = مقاطع الفيديو
finish-group-music = الموسيقى
finish-group-documents = المستندات
finish-group-other-files = ملفات أخرى
finish-group-apps = التطبيقات
finish-group-settings = الإعدادات
finish-note-parental-controls = إعدادات الرقابة الأبوية وسلامة الأسرة لا تُنقل. أعد ضبطها على هذا الحاسوب المحمول.
finish-note-not-everywhere = لم يتم البحث في بعض الأماكن على الحاسوب المحمول القديم.
finish-note-results-refused = لم تُقبل بعض النتائج، لأن هذا الحاسوب المحمول لم يُظهرها على أنها اكتملت.
finish-note-undo-open = التراجع ما زال متاحًا لعملية النقل هذه. ويُغلق نهائيًا إذا مسحت الحاسوب المحمول القديم.
finish-open-title = لم يكتمل
finish-open-body = هذه عناصرك التي ما زالت تحتاج إلى انتباه.
finish-standing-copied = تم النسخ
finish-standing-reinstalled = أُعيد تثبيته
finish-standing-converted = تم تحويله
finish-standing-needs-action = يحتاج إليك
finish-standing-failed = لم يُنسخ
finish-standing-waiting = ينتظر
finish-standing-not-reachable = الحاسوب المحمول القديم غير متاح
finish-standing-no-result-yet = لا نتيجة بعد
finish-standing-skipped-by-user = تجاوزته أنت
finish-standing-left-out = مستبعد
finish-access-key-not-checked = لم يتم التحقق هنا من المفتاح أو الرمز اللازم لفتحه.
finish-access-not-on-new-laptop = غير موجود على هذا الحاسوب المحمول.
finish-search-title = أين بحث PCTwin على الحاسوب المحمول القديم
finish-search-searched = تم البحث في: { $places }
finish-search-not-searched = لم يتم البحث في: { $places }
finish-search-all = بحث PCTwin في كل مكان يعرفه. ولا يستطيع القول إنه لا يوجد شيء في أي مكان آخر.
finish-old-checked = آخر فحص للحاسوب المحمول القديم: { $when }
finish-old-never = لم يُفحص الحاسوب المحمول القديم منذ النقل.
finish-steps-title = الخطوات التالية
finish-step-setup = أنهِ الإعداد
finish-step-setup-detail = تطبيقات تحتاج إلى تسجيل دخولك أو إلى إنهاء خطوة.
finish-step-goodbye = قبل أن تترك الحاسوب المحمول القديم
finish-step-goodbye-detail = ما ما زال موجودًا على الحاسوب المحمول القديم فقط.
finish-step-catchup = انسخ التغييرات الأخيرة
finish-step-catchup-detail = انسخ ما تغيّر على الحاسوب المحمول القديم منذ النقل.
finish-step-handover = سلّم الحاسوب المحمول القديم
finish-step-handover-detail = أزل حسابك أو امسح الحاسوب المحمول القديم، بأدوات النظام نفسه.
finish-step-undo = تراجع عن عملية النقل هذه
finish-step-undo-detail = أزل التطبيقات وأعد الإعدادات.
finish-step-cleanup = تنظيف
finish-step-cleanup-detail = أزل ملفات PCTwin المؤقتة من هذا الحاسوب المحمول.
finish-step-report = شارك التقرير
finish-step-report-detail = احفظ تقريرًا بسيطًا. أنت تختار ما يتضمنه.
finish-step-doctor = افحص الاتصال
finish-step-doctor-detail = اعرف لماذا لا يتصل الحاسوبان المحمولان.
finish-setup-title = أنهِ الإعداد
finish-setup-body = هذه التطبيقات تحتاج إلى خطوة منك. علامة الصح تسجّل ما تقوله أنت. ولا تُعدّ فحصًا من PCTwin.
finish-setup-empty = لا شيء ينتظرك.
finish-setup-sign-in = سجّل الدخول إلى { $name }
finish-setup-licence-key = أدخل مفتاح ترخيص { $name }
finish-setup-set-up-again = أعد إعداد { $name }
finish-setup-download-from-maker = نزّل { $name } من موقع الجهة المطورة
finish-setup-permission = امنح { $name } الإذن عندما يطلبه
finish-setup-restart = أعد تشغيل هذا الحاسوب المحمول لإنهاء { $name }
finish-setup-finish-by-hand = أنهِ { $name } بنفسك
finish-setup-parts-missing = لم تصل بعض أجزاء { $name }. أرسلها مرة أخرى
finish-setup-export-first = صدّر من { $name } على الحاسوب المحمول القديم أولًا
finish-setup-relink = اربط { $name } بحسابك مرة أخرى
finish-setup-look-at-it = افتح { $name } وتأكد أنه سليم
finish-setup-mark-open = لم يتم
finish-setup-mark-machine-checked = تحقق منه PCTwin
finish-setup-mark-you-did-it = قلت إنه تم
finish-setup-tick-hint = ضع علامة عندما تنتهي منه.
finish-setup-save-row = احفظ قائمة تطبيقاتك وإعداداتك
finish-setup-save-row-detail = احتفظ بقائمة تطبيقاتك وإعداداتك في سجل PCTwin الخاص بي.
finish-save-title = احفظ قائمة تطبيقاتك وإعداداتك
finish-save-body = يحتفظ PCTwin بقائمة تطبيقاتك وإعداداتك في سجل PCTwin الخاص بي، على هذا الحاسوب المحمول فقط. هي قائمة، وليست نسخة من ملفاتك.
finish-save-button = احفظ القائمة
finish-save-saved = تم الحفظ في سجل PCTwin الخاص بي. التطبيقات: { $apps }. الإعدادات: { $settings }.
finish-save-earlier-title = محفوظ سابقًا
finish-save-earlier = تم الحفظ: { $when }
finish-goodbye-title = قبل أن تترك الحاسوب المحمول القديم
finish-goodbye-body = هذا ما ما زال موجودًا على الحاسوب المحمول القديم فقط. أجب عن كل عنصر. لا يُمسح شيء هنا. المسح خطوة منفصلة.
finish-goodbye-saved = هذه القائمة محفوظة على هذا الحاسوب المحمول.
finish-goodbye-only-on-old-laptop = على الحاسوب المحمول القديم فقط: { $count }
finish-goodbye-key-not-checked = عناصر تحتاج إلى مفتاح أو رمز لم يتم التحقق منه: { $count }
finish-goodbye-tied-to-old-laptop = مرتبطة بالحاسوب المحمول القديم: { $count }
finish-goodbye-do-first-on-old-laptop = يجب فعله على الحاسوب المحمول القديم أولًا: { $count }
finish-goodbye-steps-not-finished = خطوات لم تنتهِ: { $count }
finish-goodbye-not-searched = أماكن لم يُبحث فيها: { $count }
finish-goodbye-old-laptop-not-reachable = تعذّر الوصول إلى الحاسوب المحمول القديم
finish-goodbye-special-sign-in = { $names }: سجّل الدخول مرة أخرى
finish-goodbye-special-relink = { $names }: اربطه بحسابك مرة أخرى
finish-goodbye-special-export-first = { $names }: صدّر ما تحتاجه من الحاسوب المحمول القديم أولًا
finish-goodbye-special-look-at-it = { $names }: افتحه وتأكد أنه سليم
finish-goodbye-places = الأماكن: { $places }
finish-goodbye-did-it = لقد فعلت ذلك
finish-goodbye-accept = أقبل أنه سيضيع
finish-goodbye-accept-warning = إذا قبلت ذلك، فسيبقى على الحاسوب المحمول القديم فقط. وعندما يُمسح الحاسوب المحمول القديم، يضيع.
finish-goodbye-cannot-answer = لا يحسم هذا إلا فحص جديد للحاسوب المحمول القديم. صله وافحصه مرة أخرى.
finish-goodbye-res-open = لم تتم الإجابة بعد
finish-goodbye-res-person-did-it = قلت إنك فعلت ذلك
finish-goodbye-res-accepted-loss = قبلت أنه سيضيع
finish-goodbye-answer-label = إجابتك عن: { $title }
finish-goodbye-keep-title = اترك الحاسوب المحمول القديم كما هو حتى
finish-goodbye-keep-body = التاريخ المقترح: { $date }. حتى ذلك الحين لا تمسحه، تحسبًا لحاجتك إلى شيء منه.
finish-goodbye-keep-label = احتفظ به حتى
finish-goodbye-keep-use = استخدم التاريخ المقترح
finish-goodbye-keep-save = احفظ هذا التاريخ
finish-goodbye-keep-set = محفوظ حتى: { $date }
finish-goodbye-keep-none = لم يُختر تاريخ بعد
finish-goodbye-blocked = لا يمكن تسليم الحاسوب المحمول القديم بعد. ما زال مفتوحًا: { $count }.
finish-goodbye-blocked-unchecked = لم يُفحص الحاسوب المحمول القديم منذ النقل. افحصه مرة أخرى أولًا.
finish-goodbye-clear = تمت الإجابة عن كل شيء وفُحص الحاسوب المحمول القديم. يمكنك طلب التسليم. وهي خطوة منفصلة.
finish-goodbye-handover = سلّم الحاسوب المحمول القديم
finish-goodbye-check-again = افحص الحاسوب المحمول القديم مرة أخرى
finish-catchup-title = انسخ التغييرات الأخيرة
finish-catchup-body = اطّلع على ما تغيّر في الحاسوب المحمول القديم منذ النقل. تراجع كل تغيير بنفسك. ولا يرسل PCTwin إلا ما تختاره.
finish-catchup-purpose-check = انظر إلى الحاسوب المحمول القديم فقط
finish-catchup-purpose-check-detail = لا يُنسخ شيء. تتحدث قائمة ما قبل المغادرة.
finish-catchup-purpose-catch-up = انظر وانسخ ما تغيّر
finish-catchup-purpose-catch-up-detail = ترى التغييرات أولًا وتختار ما يُرسل.
finish-catchup-armed = جاهز. صل الحاسوب المحمول القديم بالطريقة نفسها التي استخدمتها في النقل. ستعرض هذه الصفحة التغييرات عندما يستجيب الحاسوب المحمول القديم.
finish-catchup-connect = صل الحاسوب المحمول القديم
finish-catchup-waiting = بانتظار الحاسوب المحمول القديم.
finish-catchup-unreadable = لم يستطع PCTwin قراءة رد الحاسوب المحمول القديم. لم يتغير شيء. حاول مرة أخرى.
finish-catchup-changes-title = ما الذي تغيّر
finish-catchup-check-only = لا يُرسل شيء. هذه القائمة تحدّث فقط ما يعرفه PCTwin عن الحاسوب المحمول القديم.
finish-catchup-none = لم يتغير شيء على الحاسوب المحمول القديم.
finish-catchup-others = تغييرات في عناصر أشخاص آخرين، تُحسب فقط: { $count }
finish-catchup-choose-label = ماذا يفعل PCTwin مع { $title }؟
finish-catchup-send-label = أرسل { $title }
finish-catchup-nothing-to-send = لا شيء للإرسال
finish-change-new = جديد على الحاسوب المحمول القديم
finish-change-updated = تغيّر على الحاسوب المحمول القديم
finish-change-both-changed = تغيّر على الحاسوبين المحمولين
finish-change-gone-on-new-laptop = أُزيل من هذا الحاسوب المحمول
finish-change-gone-on-old-laptop = أُزيل من الحاسوب المحمول القديم
finish-change-cannot-tell = لا يستطيع PCTwin الجزم
finish-choice-none = لا ترسل
finish-choice-send = أرسله مرة أخرى
finish-choice-keep-both = احتفظ بالنسختين
finish-choice-keep-new-laptop-version = احتفظ بنسخة هذا الحاسوب المحمول
finish-catchup-approve = أرسل التغييرات التي اخترتها: { $count }
finish-catchup-sent = أُرسلت للنسخ. لا تُنسخ إلا التغييرات التي اخترتها.
finish-handover-title = سلّم الحاسوب المحمول القديم
finish-handover-body = كل خيار قرار منفصل. لا يحدث شيء حتى تؤكد في صفحته الخاصة.
finish-handover-remove-title = أزل حسابي من الحاسوب المحمول القديم
finish-handover-remove-detail = حسابك أنت فقط. حسابات الآخرين تبقى.
finish-handover-erase-title = امسح الحاسوب المحمول القديم
finish-handover-erase-detail = كل ما على محرك الحاسوب المحمول القديم.
finish-handover-preview-remove = أزل حسابي
finish-handover-preview-erase = امسح الحاسوب المحمول القديم
finish-handover-removes = ما الذي سيُزال
finish-handover-keeps = ما الذي سيبقى
finish-handover-keeps-none = لا يبقى عليه شيء.
finish-handover-route = كيف يتم ذلك
finish-handover-route-body = لا ينفذ PCTwin هذا بنفسه. تنفذه أداة النظام نفسه، ويوصلك PCTwin إليها.
finish-handover-continue = تابع إلى التأكيد
finish-handover-not-offered = هذا غير متاح الآن
finish-handover-refused-open = أمور ما زالت مفتوحة في قائمة ما قبل المغادرة: { $count }
finish-handover-refused-nothing-done = لم يتم فعل أي شيء.
finish-handover-to-list = اذهب إلى قائمة ما قبل المغادرة
finish-route-system-windows = Windows
finish-route-system-macos = macOS
finish-route-system-linux = Linux
finish-route-unknown-step = اتبع الخطوة التالية التي يعرضها النظام.
finish-hnote-ssd-not-secure-wipe = حذف الملفات على SSD ليس مسحًا آمنًا.
finish-hnote-does-not-cancel-subscriptions = إزالة الحساب لا تلغي الاشتراكات ولا تُخرجك من حساباتك في أماكن أخرى.
finish-hnote-others-stay = حسابات الجميع الآخرين تبقى كما هي.
finish-hnote-cannot-be-undone = لا يمكن التراجع عن هذا.
finish-hnote-closes-undo = المتابعة تُغلق التراجع عن عملية النقل هذه نهائيًا.
finish-hnote-sign-in-as-another-administrator = لا يستطيع الحساب إزالة نفسه. سجّل الدخول إلى الحاسوب المحمول القديم بحساب مسؤول آخر أولًا.
finish-hnote-family-group-first = إذا كان الحساب في مجموعة عائلة Microsoft، فأزله من العائلة عبر الإنترنت أولًا.
finish-hnote-not-a-certificate = يسجّل PCTwin ما طلبته وما تخبره به عمّا حدث. وهذا ليس شهادة بأن شيئًا قد مُسح.
finish-hnote-encrypt-first-helps = تشغيل تشفير المحرك قبل إعادة التعيين يجعل قراءة ما يتبقى أصعب بكثير.
finish-refused-wrong-device = هذه الخطوة غير متاحة على هذا الحاسوب المحمول.
finish-refused-undo-not-closed = التراجع ما زال متاحًا لعملية النقل هذه. يجب إغلاقه أولًا.
finish-refused-old-laptop-not-checked = آخر فحص للحاسوب القديم كان قبل أكثر من 15 دقيقة. افحصه مرة أخرى أولًا.
finish-refused-list-out-of-date = قائمة ما قبل المغادرة قديمة. افتحها مرة أخرى.
finish-refused-list-not-clear = قائمة ما قبل المغادرة ما زالت فيها أمور مفتوحة.
finish-refused-no-write-exclusion = لم يستطع PCTwin التأكد من أنه لا شيء يتغير على الحاسوب المحمول القديم الآن.
finish-refused-no-such-account = لم يُعثر على حسابك في الحاسوب المحمول القديم.
finish-refused-other-people-use-it = أشخاص آخرون يستخدمون هذا الحساب، لذلك لن يزيله PCTwin.
finish-refused-not-qualified = لا يمكن تسليم هذا الحاسوب المحمول القديم بهذه الطريقة.
finish-refused-no-fresh-confirmation = يلزم تأكيد جديد.
finish-refused-wrong-confirmation = هذا التأكيد لا يطابق ما يظهر الآن. انظر مرة أخرى.
finish-refused-changed-since-confirmed = تغيّر شيء بعد أن أكدت. انظر مرة أخرى وأكد من جديد.
finish-refused-expired = انتهت صلاحية التأكيد. انظر مرة أخرى وأكد من جديد.
finish-refused-already-used = هذا التأكيد استُخدم من قبل.
finish-refused-nothing-to-remove = لا يوجد شيء لإزالته.
finish-confirm-title-remove-my-account = تأكيد: إزالة حسابي
finish-confirm-title-erase-old-laptop = تأكيد: مسح الحاسوب المحمول القديم
finish-confirm-target = سيتغير هذا: { $target }
finish-confirm-consequences = ماذا يعني هذا
finish-confirm-tick-remove-my-account = أفهم أن الحساب { $target } سيُزال من الحاسوب المحمول القديم وأنه لا يمكن التراجع عن ذلك.
finish-confirm-tick-erase-old-laptop = أفهم أن كل ما على { $target } سيُمسح وأنه لا يمكن التراجع عن ذلك.
finish-confirm-go = أكّد وافتح أداة النظام
finish-confirm-cancel = ارجع ولا تفعل شيئًا
finish-confirm-needs-tick = يعمل الزر بعد أن تضع علامة في المربع أعلاه.
finish-started-title = اتبع خطوات النظام نفسه
finish-started-body = الخطوات أدناه من النظام وليست من PCTwin. لا يعرض PCTwin نسخة خاصة به منها.
finish-started-opened = فتح PCTwin صفحة النظام.
finish-started-not-opened = تعذّر على PCTwin فتح صفحة النظام. افتحها بنفسك واتبع الخطوات.
finish-started-undo-closed = أُغلق التراجع الآن لعملية النقل هذه.
finish-says-title = أخبر PCTwin بما حدث
finish-says-label = ماذا رأيت؟
finish-says-choose = اختر ما رأيته
finish-says-done = أنهيت الخطوات
finish-says-partly = أنهيت جزءًا من الخطوات
finish-says-cancelled-part-way = ألغيت في منتصف الطريق
finish-says-not-started = لم أبدأ
finish-says-cannot-tell = لا أستطيع الجزم
finish-says-record = سجّل ما رأيته
finish-recorded-person-says-done = تم التسجيل: قلت إنه انتهى. هذا قولك أنت. وهو ليس شهادة.
finish-recorded-partial = تم التسجيل: لم يتم منه إلا جزء.
finish-recorded-interrupted = تم التسجيل: توقف في المنتصف.
finish-recorded-cancelled-after-some-effects = تم التسجيل: أُلغي بعد إجراء بعض التغييرات.
finish-recorded-not-started = تم التسجيل: لم يبدأ.
finish-recorded-inconclusive = تم التسجيل: لم تستطع الجزم بما حدث.
finish-recorded-time = تم التسجيل: { $when }
finish-undo-title = راجع قبل التراجع
finish-undo-body = هذا ما يفعله التراجع وما يتركه. لا يحدث شيء حتى تختار بدء التراجع.
finish-undo-row-removes = التطبيقات التي ثبّتها PCTwin: يزيلها التراجع
finish-undo-row-yours = التطبيقات التي أصبحت لك الآن: يتركها التراجع. أزلها بنفسك إن أردت.
finish-undo-row-settings = إعدادات أُعيدت كما كانت: { $count }
finish-undo-row-appdata = بيانات التطبيقات أُعيدت لهذه التطبيقات
finish-undo-row-files = ملفات نسخها النقل: { $count }
finish-undo-limit-files = فقط والحاسوب المحمول القديم متصل، وفقط للنسخ التي لم يغيّرها أحد منذ ذلك الحين.
finish-undo-limit-apps = فقط ما دام كل تطبيق على الإصدار الذي ثبّته PCTwin.
finish-undo-keeps = ما يحتفظ به التراجع: التطبيقات التي أصبحت لك الآن، وكل ما غيّرته بعد النقل.
finish-undo-closed = أُغلق التراجع لعملية النقل هذه. لا يمكن استخدامه.
finish-undo-nothing = لا توجد تطبيقات أو إعدادات للتراجع عنها في عملية النقل هذه.
finish-undo-start = ابدأ التراجع
finish-cleanup-title = تنظيف
finish-cleanup-body = يُطرح كل سؤال على حدة. لا يُزال شيء ما لم تؤكد.
finish-cleanup-temp = ملفات PCTwin المؤقتة
finish-cleanup-temp-detail = ملفات أنشأها PCTwin على هذا الحاسوب المحمول أثناء عمله.
finish-cleanup-model = نموذج PCTwin وبياناته المحفوظة
finish-cleanup-model-detail = ملفات مساعدة يحتفظ بها PCTwin على هذا الحاسوب المحمول ليعمل أسرع.
finish-cleanup-look = انظر
finish-cleanup-removes = ستُزال
finish-cleanup-keeps = ستبقى
finish-cleanup-keeps-body = هذه تشبه ملفات PCTwin فقط، أو أنها قيد الاستخدام.
finish-cleanup-nothing = لا يوجد شيء لإزالته.
finish-cleanup-tick = أزل العناصر المدرجة تحت ستُزال
finish-cleanup-confirm = أزلها
finish-cleanup-done = أُزيل: { $removed }. بقي: { $kept }. تعذّرت إزالته: { $failed }.
finish-doctor-title = افحص الاتصال
finish-doctor-body = ينظر PCTwin في هذا الحاسوب المحمول ليعرف لماذا لا يتصل الحاسوبان المحمولان.
finish-doctor-role-label = أي حاسوب محمول هذا؟
finish-doctor-role-old = الحاسوب المحمول القديم
finish-doctor-role-new = الحاسوب المحمول الجديد
finish-doctor-run = ابدأ الفحص
finish-doctor-running = جارٍ فحص هذا الحاسوب المحمول.
finish-doctor-found = ما وجده PCTwin
finish-doctor-none = لم يجد PCTwin مشكلة على هذا الحاسوب المحمول. وهذا لا يثبت أن الاتصال سيعمل.
finish-doctor-sure = مؤكد
finish-doctor-likely = محتمل
finish-doctor-fix = ما العمل
finish-doctor-not-checked = لم يستطع PCTwin فحصه
finish-notchecked-unknown = تعذّر إجراء فحص واحد على هذا الحاسوب المحمول.
finish-report-title = شارك التقرير
finish-report-body = اختر ما يتضمنه التقرير. لا يُضاف شيء ما لم تختره. لا يُرسل شيء. أنت تحفظ النص الذي تقرؤه، وتشاركه بنفسك.
finish-report-choices = ضمّن في التقرير
finish-report-file-names = أسماء الملفات
finish-report-app-names = أسماء التطبيقات
finish-report-account-id = معرّف الحساب
finish-report-preview = التقرير كما سيُحفظ
finish-report-withheld = تُرك خارج التقرير: الأسماء { $names }، الأسرار { $secrets }، معرّفات الحسابات { $ids }.
finish-report-save = احفظ التقرير
finish-report-saved = تم الحفظ في: { $path }
finish-report-not-saved = لم يُحفظ أي ملف.
finish-report-feedback = لإخبار فريق PCTwin بما حدث، افتح مساعدة ثم ملاحظات. لا حاجة إلى حساب، ولا يُرسل شيء تلقائيًا.
finish-report-help = افتح المساعدة
finish-report-text-title = تقرير نقل PCTwin
finish-report-text-unnamed-file = ملف
finish-report-text-unnamed-app = تطبيق
finish-report-text-account = معرّف الحساب: { $id }
finish-report-text-others = عناصر أشخاص آخرين لم تنتهِ، تُحسب فقط: { $count }
finish-report-text-goodbye = قائمة ما قبل المغادرة: { $total } أمور، { $open } ما زالت مفتوحة
finish-report-text-lines = العناصر التي لم تنتهِ
rescue-eyebrow = إنقاذ حاسوب
rescue-title = إنقاذ حاسوب
rescue-lead = استرجع الملفات من حاسوب لا يعمل. صِل قرصه بهذا الحاسوب، ثم اختره هنا.
rescue-checking = جارٍ البحث عن الأقراص الموصولة.
rescue-failed = لم يتمكن PCTwin من عرض الأقراص الموصولة بهذا الحاسوب. تحقق من الكابل، ثم تحقق مرة أخرى.
rescue-empty = لا يوجد قرص موصول. صِل قرص الحاسوب القديم بمحوّل USB، ثم تحقق مرة أخرى.
rescue-looks-like-laptop = يبدو أنه قرص حاسوب
rescue-choose = اختر { $name }
rescue-check-again = تحقق مرة أخرى
rescue-locked-title = إذا كان القرص مقفلًا
rescue-locked-windows = إذا كان القرص مقفلًا بـ BitLocker، يطلب Windows مفتاح الاسترداد في نافذته الخاصة عند فتح القرص.
rescue-wait-title = جارٍ قراءة { $drive }
rescue-wait-body = يفحص PCTwin المجلدات على { $drive }. لا يتغير شيء على القرص.
rescue-accept-title = ملفات وُجدت على { $drive }
rescue-accept-body = اقبل، وسينسخها PCTwin إلى هذا الحاسوب. لا يتغير شيء على القرص.
rescue-progress-title = جارٍ نسخ الملفات من { $drive }
rescue-safe = القرص يُقرأ فقط. لا يتغير شيء عليه.
rescue-outcome-title = ملفات نُسخت من { $drive }
plan-net-title = ما يحتاج إلى الإنترنت
plan-net-none = لا شيء في هذا النقل يحتاج إلى الإنترنت.
plan-net-downloads =
    { $count ->
        [zero] ينزّل PCTwin { $count } تطبيق من الإنترنت.
        [one] ينزّل PCTwin تطبيق واحد من الإنترنت.
        [two] ينزّل PCTwin تطبيقان من الإنترنت.
        [few] ينزّل PCTwin { $count } تطبيقات من الإنترنت.
        [many] ينزّل PCTwin { $count } تطبيقًا من الإنترنت.
       *[other] ينزّل PCTwin { $count } تطبيق من الإنترنت.
    }
plan-net-sizes = لا يُعرف حجم كل تنزيل إلا بعد انتهائه.
plan-net-signins =
    { $count ->
        [zero] ستسجّل الدخول من جديد عبر الإنترنت إلى { $count } شيء.
        [one] ستسجّل الدخول من جديد عبر الإنترنت إلى شيء واحد.
        [two] ستسجّل الدخول من جديد عبر الإنترنت إلى شيئان.
        [few] ستسجّل الدخول من جديد عبر الإنترنت إلى { $count } أشياء.
        [many] ستسجّل الدخول من جديد عبر الإنترنت إلى { $count } شيئًا.
       *[other] ستسجّل الدخول من جديد عبر الإنترنت إلى { $count } شيء.
    }
plan-room-title = المساحة اللازمة فوق ملفاتك
plan-room-body = في أكثر اللحظات ازدحامًا يحتاج PCTwin إلى نحو { $installs } للتطبيقات ونحو { $appData } لبيانات التطبيقات، فوق ملفاتك.
compat-strip = ما جرى اختباره
compat-title = { $name }: ما جرى اختباره
compat-open = التفاصيل
compat-open-name = عرض ما جرى اختباره في { $name }
compat-list-title =
    { $count ->
        [zero] نتيجة اختبار { $count } تطبيق
        [one] نتيجة اختبار تطبيق واحد
        [two] نتيجة اختبار تطبيقان
        [few] نتيجة اختبار { $count } تطبيقات
        [many] نتيجة اختبار { $count } تطبيقًا
       *[other] نتيجة اختبار { $count } تطبيق
    }
compat-aspects-title = كل جزء
compat-steps-title = ما ستفعله أنت
compat-steps-none = لا خطوات مطلوبة منك.
compat-never-checked = لم يختبر أحد هذا التطبيق بعد. سيجرّبه PCTwin ويخبرك بما حدث.
compat-checked-on = جرى الفحص في { $date }.
compat-version = الإصدار الذي جرى اختباره: { $version }
work-strip = فحص العمل
work-title = تحقق من أنك تستطيع العمل هنا
work-body = أضف الملفات والخطوط والمهام التي تحتاجها على هذا الحاسوب. ينظر PCTwin في ما يستطيع قراءته، وأنت تقول هل نجح الباقي. لا يُفتح شيء ولا يُشغَّل.
work-empty = لا توجد فحوصات بعد. أضف ملفًا أو خطًّا أو مهمة في الأسفل.
work-looked-at = جرى النظر في { $checked } من { $of }.
work-missing = ناقص:
work-and-more =
    { $count ->
        [zero] وغيرها: { $count }.
        [one] وغيرها: { $count }.
        [two] وغيرها: { $count }.
        [few] وغيرها: { $count }.
        [many] وغيرها: { $count }.
       *[other] وغيرها: { $count }.
    }
work-not-looked-at = لم يجرِ النظر فيه:
work-answer-worked = نجح
work-answer-worked-name = نجح: { $name }
work-answer-not = لم ينجح
work-answer-not-name = لم ينجح: { $name }
work-remove = إزالة
work-remove-name = إزالة { $name }
work-run = افحص الآن
work-add-title = إضافة فحص
work-add-file = اختيار ملف
work-add-file-hint = مشروع أو أرشيف أو قاعدة بيانات. يقرؤه PCTwin ولا يغيّره أبدًا.
work-font-label = خط، بالاسم
work-add-font = إضافة الخط
work-task-label = مهمة تؤديها بنفسك
work-task-hint = مثلًا: طباعة بطاقة صعود الطائرة. أنت تقول هل نجحت.
work-add-task = إضافة المهمة
repair-strip = إصلاح ما لم يكتمل
repair-title =
    { $count ->
        [zero] إصلاح { $count } عنصر
        [one] إصلاح عنصر واحد
        [two] إصلاح عنصران
        [few] إصلاح { $count } عناصر
        [many] إصلاح { $count } عنصرًا
       *[other] إصلاح { $count } عنصر
    }
repair-title-loading = إصلاح ما لم يكتمل
repair-body =
    { $kept ->
        [zero] تُعاد محاولة العناصر أدناه فقط. تبقى كما هي { $kept } عنصر التي نجحت سابقًا.
        [one] تُعاد محاولة العناصر أدناه فقط. تبقى كما هي عنصر واحد التي نجحت سابقًا.
        [two] تُعاد محاولة العناصر أدناه فقط. تبقى كما هي عنصران التي نجحت سابقًا.
        [few] تُعاد محاولة العناصر أدناه فقط. تبقى كما هي { $kept } عناصر التي نجحت سابقًا.
        [many] تُعاد محاولة العناصر أدناه فقط. تبقى كما هي { $kept } عنصرًا التي نجحت سابقًا.
       *[other] تُعاد محاولة العناصر أدناه فقط. تبقى كما هي { $kept } عنصر التي نجحت سابقًا.
    }
repair-nothing = لا شيء يحتاج إلى إصلاح.
repair-can-now = يمكن تجربته الآن
repair-waiting = في الانتظار
repair-fixed =
    { $count ->
        [zero] تم إصلاح { $count } عنصر.
        [one] تم إصلاح عنصر واحد.
        [two] تم إصلاح عنصران.
        [few] تم إصلاح { $count } عناصر.
        [many] تم إصلاح { $count } عنصرًا.
       *[other] تم إصلاح { $count } عنصر.
    }
repair-run =
    { $count ->
        [zero] أعد محاولة { $count } عنصر
        [one] أعد محاولة عنصر واحد
        [two] أعد محاولة عنصران
        [few] أعد محاولة { $count } عناصر
        [many] أعد محاولة { $count } عنصرًا
       *[other] أعد محاولة { $count } عنصر
    }
changed-strip = تغييرات منذ موافقتك
changed-title =
    { $count ->
        [zero] تغيّر { $count } شيء بعد موافقتك
        [one] تغيّر شيء واحد بعد موافقتك
        [two] تغيّر شيئان بعد موافقتك
        [few] تغيّر { $count } أشياء بعد موافقتك
        [many] تغيّر { $count } شيئًا بعد موافقتك
       *[other] تغيّر { $count } شيء بعد موافقتك
    }
changed-body = هذه فقط تحتاج إلى موافقتك مرة أخرى. كل ما عداها يبقى موافقًا عليه.
changed-affects = يؤثر في: { $steps }
changed-held =
    { $count ->
        [zero] تنتظر { $count } خطوة ردّك.
        [one] تنتظر خطوة واحدة ردّك.
        [two] تنتظر خطوتان ردّك.
        [few] تنتظر { $count } خطوات ردّك.
        [many] تنتظر { $count } خطوة ردّك.
       *[other] تنتظر { $count } خطوة ردّك.
    }
changed-leave-out = استبعاد هذه
changed-approve = وافق مرة أخرى
safe-stop-strip = التحديث
safe-stop-title = في انتظار لحظة آمنة
safe-stop-body = لا يحدّث PCTwin نفسه ولا يزيل نفسه في منتصف النقل. ينتظر حتى لا يبقى شيء نصف منجز.
safe-stop-failed = لم يستطع PCTwin معرفة ما إذا كان الأمر آمنًا. حاول مرة أخرى.
safe-stop-ready = لا شيء يتغيّر الآن في منتصف العمل. يمكن لـ PCTwin أن يتحدّث أو يُزال الآن.
safe-stop-waiting-for = ينتظر انتهاء هذه الأمور:
safe-stop-nothing-going-on = لا يجري نقل ولا إعداد ولا تراجع.
safe-stop-check = افحص مرة أخرى
# Help feedback, safe stop links, catch-up and repair routes
feedback-strip = ملاحظات
feedback-title = كيف سارت عملية النقل؟
feedback-body = الملاحظات اختيارية ولا تؤثر في عمليات النقل المجانية.
feedback-rating-label = التقييم
feedback-rating-none = بدون تقييم
feedback-rating-very-good = ممتاز
feedback-rating-good = جيد
feedback-rating-needs-work = يحتاج إلى تحسين
feedback-words-label = ما الذي سيجعل هذا أفضل؟
feedback-words-hint = اختياري. لا تكتب أسماء أو كلمات مرور أو أسماء ملفات.
feedback-preview-title = هذا كل ما سيتم مشاركته
feedback-preview-rating = التقييم: { $rating }
feedback-preview-empty = لا شيء بعد. اختر تقييمًا أو اكتب بضع كلمات.
feedback-sends-nothing = لا يرسل PCTwin شيئًا من تلقاء نفسه. لا حاجة إلى حساب. لمشاركة هذا النص، انسخه ثم افتح getpctwin.com للتواصل مع فريق PCTwin.
feedback-copy = انسخ الملاحظات
feedback-copied = تم نسخ الملاحظات.
feedback-copy-failed = تعذّر نسخ الملاحظات. حدّد النص في الأعلى وانسخه بنفسك.
feedback-open-site = افتح getpctwin.com
help-home-feedback = ملاحظات
help-home-update = تحديث PCTwin أو إزالته
safe-stop-see = اعرض ما ينتظره
history-repair = إصلاح العناصر
kit-strip = حزمة الاسترداد
kit-title = حفظ حزمة استرداد
kit-body = حزمة الاسترداد سجلّ صغير لهذا النقل، يُحفظ في مجلد تختاره، مثل قرص USB. تُظهر ما حدث وما بقي دون إنجاز، حتى من دون هذا الحاسوب.
kit-lacks-sentence = لا تحتوي الحزمة على ملفاتك نفسها، ولا تستطيع إعادة ملفات مفقودة.
kit-not-saved = لم يُختر أي مجلد. لم يُحفظ شيء.
kit-saved-to = حُفظت في:
kit-files-title = ملفات الحزمة
kit-holds-title = ما تحتويه الحزمة
kit-lacks-title = ما لا تحتويه الحزمة
kit-save = اختر مجلدًا واحفظ
kit-save-again = احفظ مرة أخرى
kit-history = حزمة الاسترداد
finish-step-repair =
    { $count ->
        [zero] إصلاح { $count } عنصر
        [one] إصلاح عنصر واحد
        [two] إصلاح عنصران
        [few] إصلاح { $count } عناصر
        [many] إصلاح { $count } عنصرًا
       *[other] إصلاح { $count } عنصر
    }
finish-step-repair-detail = أعد محاولة ما لم يكتمل فقط. ما نجح يبقى كما هو.
finish-step-work = تحقق من أنك تستطيع العمل هنا
finish-step-work-detail = تأكد أن الملفات والخطوط والمهام التي تحتاجها موجودة هنا.
finish-step-kit = حفظ حزمة استرداد
finish-step-kit-detail = احتفظ بسجلّ صغير لهذا النقل في مجلد تختاره.
space-box-title = لا توجد مساحة كافية على هذا الحاسوب لكل شيء
space-box-choice = { $count ->
        [zero] لا ملفات لتركها على حاسوبك القديم
        [one] اترك على حاسوبك القديم ملفًا واحدًا ({ $size }) لم تستخدمه منذ أطول مدة
        [two] اترك على حاسوبك القديم ملفين ({ $size }) لم تستخدمهما منذ أطول مدة
        [few] اترك على حاسوبك القديم { $count } ملفات ({ $size }) لم تستخدمها منذ أطول مدة
        [many] اترك على حاسوبك القديم { $count } ملفًا ({ $size }) لم تستخدمها منذ أطول مدة
       *[other] اترك على حاسوبك القديم { $count } ملف ({ $size }) لم تستخدمها منذ أطول مدة
    }
space-box-kept = لا يُحذف شيء. تبقى على حاسوبك القديم.
space-box-short = حتى مع ذلك، ما زال ينقص { $size }. ألغِ تحديد بعض المجلدات، أو اختر مكانًا آخر لها.
space-box-fits = مع بقائها على حاسوبك القديم، يتسع كل ما تبقى.
finish-handover-check-old = افحص الحاسوب القديم مرة أخرى
finish-handover-check-armed = PCTwin جاهز لفحص الحاسوب القديم. افتح PCTwin على الحاسوب القديم وصِل الحاسوبين كما في النقل. ثم اختر انظر مرة أخرى.
finish-handover-look-again = انظر مرة أخرى
qs-where-title = أين تُحفظ هذه الملفات؟
qs-where-body = { $count ->
        [zero] لا ملفات، { $size }. لا يُحفظ شيء حتى تختار.
        [one] ملف واحد، { $size }. لا يُحفظ شيء حتى تختار.
        [two] ملفان، { $size }. لا يُحفظ شيء حتى تختار.
        [few] { $count } ملفات، { $size }. لا يُحفظ شيء حتى تختار.
        [many] { $count } ملفًا، { $size }. لا يُحفظ شيء حتى تختار.
       *[other] { $count } ملف، { $size }. لا يُحفظ شيء حتى تختار.
    }
qs-where-folder = المجلد: { $folder }
qs-where-details = مزيد من التفاصيل
qs-where-path = المكان الكامل: { $path }
qs-where-save = احفظ هنا
qs-where-choose = اختر مجلدًا آخر
qs-where-stop = إيقاف

# 2.9 destination changed after approval
outcome-landed-moved = نُقل هذا المجلد أثناء النقل. ملفاتك فيه، أينما كان الآن.
move-place-changed-title = تغيّر مجلد بعد موافقتك
move-place-changed-body = نُقل مجلد كانت الملفات ستُنسخ إليه أو استُبدل بعد موافقتك على النقل. لم يُنسخ أي شيء. تحقق من المجلد، ثم ابدأ النقل من جديد.

# Quick Send stage 1
qs-refusal-not-paired = لم يعد الجهازان متصلين. صِلهما مرة أخرى.
qs-refusal-not-waiting = لا يوجد شيء ينتظر هذا الآن.
qs-refusal-failed = لم ينجح ذلك. حاول مرة أخرى.
qs-refusal-note-empty = اكتب شيئًا لإرساله أولًا.
qs-refusal-note-too-long = هذه الملاحظة طويلة جدًا. اجعلها أقل من 1000 حرف.
qs-refusal-note-too-many-lines = في هذه الملاحظة أسطر كثيرة جدًا. اجعلها 20 سطرًا على الأكثر.
qs-refusal-note-hidden = في هذه الملاحظة حرف لا يمكن عرضه بأمان. اكتبها مرة أخرى.
qs-refusal-no-link = لا يوجد رابط لفتحه.
qs-refusal-not-found = لم يعد هذا موجودًا.
qs-refusal-restored = تمت استعادة هذا الجهاز أو نسخه. صِل الجهازين مرة أخرى برمز.
qs-refusal-bad-name = اختر اسمًا من 40 حرفًا على الأكثر.
qs-refusal-store-unavailable = تعذّر على PCTwin الوصول إلى المكان الآمن للمفاتيح في هذا الجهاز. حاول مرة أخرى بعد تسجيل الدخول.
qs-refusal-too-many-connections = لديك اتصالات محفوظة كثيرة جدًا. انسَ واحدًا أولًا.
qs-refusal-folder-runs-at-sign-in = هذا المجلد يشغّل برامج عند تسجيل الدخول. اختر مجلدًا آخر.
qs-refusal-folder-shell-start-up = هذا المجلد يشغّل أشياء عند فتح الطرفية. اختر مجلدًا آخر.
qs-refusal-folder-holds-keys = هذا المجلد يحفظ مفاتيح. اختر مجلدًا آخر.
qs-refusal-folder-system = هذا من مجلدات النظام نفسه. اختر مجلدًا آخر.
qs-refusal-folder-not-plain = هذا ليس مجلدًا عاديًا. اختر مجلدًا آخر.
qs-refusal-folder-not-the-same = هذا ليس المجلد الذي اخترته من قبل. اختره مرة أخرى.
qs-refusal-not-nearby = الجهاز الآخر ليس قريبًا. ربما يكون مطفأً أو على شبكة أخرى.
qs-refusal-over-cap = الإرسالات المنتظرة ستشغل مساحة كبيرة جدًا. ألغِ واحدًا أولًا.
qs-refusal-no-room = لا توجد مساحة فارغة كافية على هذا الجهاز.
qs-refusal-bad-days = اختر من 1 إلى 30 يومًا.
qs-refusal-bad-recipients = اختر كل شخص مرة واحدة، حتى 20.
qs-refusal-too-many-files = الملفات كثيرة جدًا. اختر عددًا أقل.
qs-refusal-changed-while-copying = تغيّر ملف أثناء نسخه. حاول مرة أخرى.
qs-note-copy = نسخ
qs-note-open = فتح
qs-note-link-to = هذا الرابط يذهب إلى { $host }.
qs-note-sent = وصلت الملاحظة.
qs-remember-ask = هل تريد تذكّر هذا الاتصال؟
qs-remember-body = في المرة القادمة يمكنك الإرسال دون رمز. هذا يعني الجهاز نفسه كما من قبل، لا شخصًا.
qs-remember-fallback = على هذا الجهاز يُحفظ المفتاح في ملف داخل حسابك.
qs-known-same-device = الجهاز نفسه كما من قبل
qs-known-restored = تمت استعادة هذا الجهاز أو نسخه. صِل مرة أخرى.
qs-known-last-used = آخر استخدام: { $date }
qs-known-forget = نسيان
qs-known-forget-body = يُحذف المفتاح وكل الأذونات. تبقى الملفات المرسلة من قبل في مكانها.
qs-known-allow-folder = السماح بالإرسالات القادمة إلى مجلد
qs-known-allow-body = تذهب الملفات إلى مجلد اسمه PCTwin Quick Send داخله. ينتهي هذا وحده بعد 30 يومًا.
qs-known-wants-to-send = { $label } يريد أن يرسل إليك هذا.
qs-known-delivered = وصلت ملفات من { $label } إلى { $folder }.
qs-waiting-days =
    { $count ->
        [zero] ينتظر { $count } يوم
        [one] ينتظر يومًا واحدًا
        [two] ينتظر يومين
        [few] ينتظر { $count } أيام
        [many] ينتظر { $count } يومًا
       *[other] ينتظر { $count } يوم
    }
qs-waiting-files =
    { $count ->
        [zero] لا ملفات في الانتظار
        [one] ملف واحد في الانتظار
        [two] ملفان في الانتظار
        [few] { $count } ملفات في الانتظار
        [many] { $count } ملفًا في الانتظار
       *[other] { $count } ملف في الانتظار
    }
qs-waiting-newer = توجد نسخة أحدث. راجعها مرة أخرى.
qs-waiting-on = لا يُرسل إلا وهذا الجهاز يعمل.
qs-waiting-cancel = إلغاء وحذف النسخة

# Quick Send stage 1 fixes
qs-known-cannot-tell = لا يستطيع PCTwin معرفة ما إذا كان هذا هو الجهاز نفسه.
qs-remember-replaces = هل سبق أن وصلت هذا الجهاز؟ اختره، وسيُنسى الاتصال القديم.
qs-remember-replaces-suggested = قد يكون هذا { $label } مرة أخرى. هل تريد استبداله؟
qs-remember-replace = استبدال
qs-remember-keep-both = لا، احتفظ بالاثنين

# 2.6 wiring
reason-open-in-app = كان أحد التطبيقات يفتح هذا الملف على الحاسوب المحمول القديم، لذا لم ينسخه PCTwin (قد تتلف النسخة إذا أُخذت أثناء كتابة التطبيق فيه). أغلق ذلك التطبيق على الحاسوب القديم، ثم ابدأ نقلًا جديدًا يشمل هذا الملف.
report-open-in = مفتوح في { $app }
fullmove-close-files = هذه الملفات مفتوحة في تطبيق على هذا الحاسوب، لذا لا يستطيع PCTwin نسخها بأمان بعد. أغلق التطبيق الذي يستخدمها، ثم اختر حاول مرة أخرى. إذا تابعت بدونها، تبقى على هذا الحاسوب ويذكرها التقرير.
choose-online-only =
    { NUMBER($count) ->
        [zero] أحضر أيضًا الملفات المحفوظة على الإنترنت فقط في { $provider } ({ $count } ملف، { $size }). تُنزَّل على هذا الحاسوب أولًا، ثم تعود إلى الإنترنت فقط.
        [one] أحضر أيضًا الملف المحفوظ على الإنترنت فقط في { $provider } (ملف واحد، { $size }). يُنزَّل على هذا الحاسوب أولًا، ثم يعود إلى الإنترنت فقط.
        [two] أحضر أيضًا الملفين المحفوظين على الإنترنت فقط في { $provider } (ملفان، { $size }). يُنزَّلان على هذا الحاسوب أولًا، ثم يعودان إلى الإنترنت فقط.
        [few] أحضر أيضًا الملفات المحفوظة على الإنترنت فقط في { $provider } ({ $count } ملفات، { $size }). تُنزَّل على هذا الحاسوب أولًا، ثم تعود إلى الإنترنت فقط.
        [many] أحضر أيضًا الملفات المحفوظة على الإنترنت فقط في { $provider } ({ $count } ملفًا، { $size }). تُنزَّل على هذا الحاسوب أولًا، ثم تعود إلى الإنترنت فقط.
       *[other] أحضر أيضًا الملفات المحفوظة على الإنترنت فقط في { $provider } ({ $count } ملف، { $size }). تُنزَّل على هذا الحاسوب أولًا، ثم تعود إلى الإنترنت فقط.
    }
choose-online-only-unnamed = خدمة تخزين على الإنترنت على هذا الحاسوب
choose-online-only-metered = أيضًا على اتصال قد يحتسب رسومًا على البيانات (نقطة اتصال هاتف)
choose-online-only-not-here = إذا كانت بعض ملفاتك محفوظة على الإنترنت فقط، فإنها تبقى في مساحة التخزين على الإنترنت. لا يستطيع PCTwin بعد إحضار نسخة منها من هذا الحاسوب.
kept-online-copied =
    { NUMBER($count) ->
        [zero] لم يُنزَّل ولم يُنسخ أي ملف محفوظ على الإنترنت فقط.
        [one] نُزِّل ونُسخ ملف واحد محفوظ على الإنترنت فقط.
        [two] نُزِّل ونُسخ ملفان محفوظان على الإنترنت فقط.
        [few] نُزِّلت ونُسخت { $count } ملفات محفوظة على الإنترنت فقط.
        [many] نُزِّل ونُسخ { $count } ملفًا محفوظًا على الإنترنت فقط.
       *[other] نُزِّل ونُسخ { $count } ملف محفوظ على الإنترنت فقط.
    }
kept-online-safe =
    { NUMBER($count) ->
        [zero] لم يبقَ أي ملف محفوظ على الإنترنت فقط دون نسخ.
        [one] لم يُنسخ ملف واحد محفوظ على الإنترنت فقط. ما زال محفوظًا على الإنترنت: افتحه على الحاسوب الجديد من التطبيق الذي يحفظه.
        [two] لم يُنسخ ملفان محفوظان على الإنترنت فقط. ما زالا محفوظين على الإنترنت: افتحهما على الحاسوب الجديد من التطبيق الذي يحفظهما.
        [few] لم تُنسخ { $count } ملفات محفوظة على الإنترنت فقط. ما زالت محفوظة على الإنترنت: افتحها على الحاسوب الجديد من التطبيق الذي يحفظها.
        [many] لم يُنسخ { $count } ملفًا محفوظًا على الإنترنت فقط. ما زالت محفوظة على الإنترنت: افتحها على الحاسوب الجديد من التطبيق الذي يحفظها.
       *[other] لم يُنسخ { $count } ملف محفوظ على الإنترنت فقط. ما زالت محفوظة على الإنترنت: افتحها على الحاسوب الجديد من التطبيق الذي يحفظها.
    }
kept-online-still-here =
    { NUMBER($count) ->
        [zero] لا يشغل أي ملف مُنزَّل مساحة إضافية على هذا الحاسوب.
        [one] ما زال ملف مُنزَّل واحد يشغل { $size } على هذا الحاسوب، لأن تطبيقه السحابي لم يُعِده إلى الإنترنت فقط بعد. لتحرير المساحة، انقر عليه بزر الفأرة الأيمن واختر تحرير المساحة.
        [two] ما زال ملفان مُنزَّلان يشغلان { $size } على هذا الحاسوب، لأن تطبيقهما السحابي لم يُعِدهما إلى الإنترنت فقط بعد. لتحرير المساحة، انقر على كل منهما بزر الفأرة الأيمن واختر تحرير المساحة.
        [few] ما زالت { $count } ملفات مُنزَّلة تشغل { $size } على هذا الحاسوب، لأن تطبيقها السحابي لم يُعِدها إلى الإنترنت فقط بعد. لتحرير المساحة، انقر على كل منها بزر الفأرة الأيمن واختر تحرير المساحة.
        [many] ما زال { $count } ملفًا مُنزَّلًا يشغل { $size } على هذا الحاسوب، لأن تطبيقها السحابي لم يُعِدها إلى الإنترنت فقط بعد. لتحرير المساحة، انقر على كل منها بزر الفأرة الأيمن واختر تحرير المساحة.
       *[other] ما زال { $count } ملف مُنزَّل يشغل { $size } على هذا الحاسوب، لأن تطبيقها السحابي لم يُعِدها إلى الإنترنت فقط بعد. لتحرير المساحة، انقر على كل منها بزر الفأرة الأيمن واختر تحرير المساحة.
    }
kept-online-paused-metered = توقف التنزيل لأن هذا الاتصال قد يحتسب رسومًا على البيانات. لم يُنسخ الباقي وما زال محفوظًا على الإنترنت.
kept-online-paused-space = توقف التنزيل لأن المساحة على هذا الحاسوب تكاد تنفد. لم يُنسخ الباقي وما زال محفوظًا على الإنترنت.
plan-own-folder = ضع أغراض { $laptop } في مجلد خاص بها
plan-own-folder-help = على هذا الحاسوب أغراض من حاسوب قديم آخر بالفعل. مجلد لكل حاسوب يُبقيها منفصلة.
plan-own-folder-name = من { $laptop }
plan-own-folder-name-unnamed = من الحاسوب الآخر
plan-downloads =
    { NUMBER($count) ->
        [zero] لن يُنزَّل أي تطبيق من الإنترنت.
        [one] يُنزَّل تطبيق واحد من الإنترنت لتثبيته على هذا الحاسوب.
        [two] يُنزَّل تطبيقان من الإنترنت لتثبيتهما على هذا الحاسوب.
        [few] تُنزَّل { $count } تطبيقات من الإنترنت لتثبيتها على هذا الحاسوب.
        [many] يُنزَّل { $count } تطبيقًا من الإنترنت لتثبيتها على هذا الحاسوب.
       *[other] يُنزَّل { $count } تطبيق من الإنترنت لتثبيتها على هذا الحاسوب.
    }
plan-wait-free-wifi = انتظر شبكة Wi-Fi مجانية قبل التنزيل
plan-wait-free-wifi-help = إذا كان الاتصال قد يحتسب رسومًا على البيانات عند تنزيل التطبيقات، ينتظر PCTwin حتى 20 دقيقة لشبكة Wi-Fi مجانية. تُدرج التطبيقات التي لم يستطع تنزيلها لتثبيتها لاحقًا.
plan-reuse-installers =
    { NUMBER($count) ->
        [zero] لا توجد برامج تثبيت من الحاسوب القديم.
        [one] استخدم برنامج التثبيت الموجود على الحاسوب القديم لتطبيق واحد، بدلًا من تنزيله
        [two] استخدم برامج التثبيت الموجودة على الحاسوب القديم لتطبيقين، بدلًا من تنزيلهما
        [few] استخدم برامج التثبيت الموجودة على الحاسوب القديم لـ { $count } تطبيقات، بدلًا من تنزيلها
        [many] استخدم برامج التثبيت الموجودة على الحاسوب القديم لـ { $count } تطبيقًا، بدلًا من تنزيلها
       *[other] استخدم برامج التثبيت الموجودة على الحاسوب القديم لـ { $count } تطبيق، بدلًا من تنزيلها
    }
plan-reuse-installers-help = يُفحص كل واحد أولًا، كما يُفحص أي تنزيل: يجب أن يكون موقّعًا من صانع التطبيق نفسه.
fullmove-close-files-title = أغلق التطبيقات التي تستخدم هذه الملفات
choose-online-only-title = الملفات المحفوظة على الإنترنت فقط
kept-online-title = الملفات المحفوظة على الإنترنت فقط
kept-online-downloaded = نُزِّل { $size }
plan-own-folder-called = اسم المجلد { $folder }.
accept-own-folder = تذهب أشياء الحاسوب الآخر إلى مجلد خاص بها، { $folder }. يمكنك إيقاف ذلك من التفاصيل.
plan-downloads-title = تطبيقات من الإنترنت
plan-connection-free = اتصال هذا الحاسوب لا يحتسب رسومًا على البيانات الآن.
plan-connection-may-charge = قد يحتسب اتصال هذا الحاسوب رسومًا على البيانات الآن، مثل نقطة اتصال من هاتف.
plan-connection-not-known = لا يستطيع PCTwin معرفة ما إذا كان اتصال هذا الحاسوب يحتسب رسومًا على البيانات.
fullmove-rebuilding-wait-wifi = إذا كان هذا الاتصال قد يحتسب رسومًا على البيانات، ينتظر PCTwin حتى 20 دقيقة لشبكة Wi-Fi مجانية قبل تنزيل التطبيقات. اتصل بشبكة Wi-Fi مجانية ليتابع أسرع.

# Quick Send stage 1 screens
qs-note-card-title = أرسل ملاحظة أو رابطًا
qs-note-card-body = بضعة أسطر من النص، أو رابط ويب واحد.
qs-home-known = الاتصالات المحفوظة
qs-home-waiting = إرسالات في الانتظار
qs-note-title = أرسل ملاحظة أو رابطًا
qs-note-intro = اكتب بضعة أسطر، أو الصق رابط ويب واحدًا. لا يُرسل إلا ما تكتبه هنا.
qs-note-label = ملاحظة أو رابط
qs-note-hint = حتى 20 سطرًا و1000 حرف.
qs-note-never-opens = لا يُفتح الرابط الذي ترسله إلا إذا اختار من يستلمه فتح.
qs-note-sending-title = جارٍ إرسال الملاحظة
qs-note-sending-body = يُرجى الانتظار حتى يستلمها الحاسوب المستلم.
qs-note-sent-title = وصلت الملاحظة
qs-note-sent-body = وصلت إلى الحاسوب المستلم. لا يعرف PCTwin هل قُرئت.
qs-note-received-title = وصلت ملاحظة
qs-note-received-link-title = وصل رابط
qs-note-from = من { $label }.
qs-note-from-code = من الحاسوب الذي وصلته برمز.
qs-note-link-safety = لا يُفتح في متصفحك إلا إذا اخترت فتح.
qs-note-copied = تم النسخ.
qs-note-copy-failed = تعذّر على PCTwin نسخه. حدّد النص وانسخه بنفسك.
qs-remember-name = اسم هذا الاتصال
qs-remember-name-hint = حتى 40 حرفًا، لتجده لاحقًا.
qs-remember-save = احفظ
qs-remember-done = حُفظ باسم { $label }. ما زالت ملفاته تسألك أولًا.
qs-known-title = الاتصالات المحفوظة
qs-known-intro = حواسيب يمكنك الإرسال إليها بلا رمز. كل واحد يعني الحاسوب نفسه كما كان، لا شخصًا.
qs-known-empty = لا توجد اتصالات محفوظة بعد. بعد الإرسال أو الاستلام برمز، يسألك PCTwin هل تريد حفظ ذلك الحاسوب.
qs-known-expires = يُنسى وحده في { $date } إن لم يُستخدم قبل ذلك.
qs-known-rename = إعادة تسمية
qs-known-rename-label = الاسم الجديد
qs-known-rename-save = احفظ الاسم
qs-known-forget-title = هل تنسى { $label }؟
qs-known-forget-confirm = انسه
qs-known-folder-on = تذهب ملفاته إلى { $folder }، في مجلد اسمه PCTwin Quick Send.
qs-known-folder-ends = ينتهي هذا في { $date }.
qs-known-folder-ended = انتهى هذا في { $date }. عادت ملفاته تسألك أولًا.
qs-known-renew = جدّد 30 يومًا
qs-known-next-title = مجلد المرة القادمة
qs-known-next-on = يكون { $folder } مختارًا مسبقًا عندما يرسل إليك ملفات. ما زال بإمكانك تغييره كل مرة.
qs-known-next-none = غير محدد. عندما يرسل إليك ملفات، يكون مجلد التنزيلات مختارًا مسبقًا، أو تختار مجلدًا آخر كل مرة.
qs-known-next-use = استخدم آخر مجلد
qs-known-next-clear = توقف عن استخدامه
qs-known-in-more = و{ $count } أخرى
qs-known-in-next = استخدم هذا المجلد لـ { $label } في المرة القادمة
qs-known-in-decline = ارفض
qs-known-delivered-counts = نُسخ { $arrived } من { $total } وتم التحقق منها.
qs-send-to = أرسل إلى { $name }
qs-to-title = من يجب أن يستلمها؟
qs-to-intro = اختر حواسيب محفوظة، أو وصّل حاسوبًا برمز.
qs-to-known = الحواسيب المحفوظة
qs-to-code = استخدم رمزًا بدلًا من ذلك
qs-to-when = متى
qs-to-now = الآن
qs-to-now-hint = يجب أن يكون الحاسوب المستلم مشغّلًا وقريبًا.
qs-to-later = عندما يكون قريبًا
qs-to-several = يستلمها كل حاسوب عندما يكون قريبًا.
qs-to-days = مدة الانتظار
qs-to-each-asks = يُسأل الحاسوب المستلم أين يحفظ، إلا إذا سمح بالإرسال إلى مجلد.
qs-to-approve-hint = أنت توافق على هذه الملفات كما هي الآن، وهذه الحواسيب وهذه المدة. يحتفظ PCTwin بنسخة من الملفات لإرسالها. يُسأل كل حاسوب أين يحفظ، إلا إذا سمح بالإرسال إلى مجلد.
qs-to-send-now = أرسل الآن
qs-to-approve = وافق على هذا التسليم
qs-waiting-title = إرسالات في الانتظار
qs-waiting-intro = ملفات يحتفظ PCTwin بنسخة منها، لإرسالها عندما يكون كل حاسوب قريبًا.
qs-waiting-empty = لا شيء في الانتظار.
qs-waiting-approved = تمت الموافقة في { $date }
qs-waiting-until = ينتظر حتى { $date }
qs-waiting-ended = انتهى في { $date }. لن يُرسل شيء آخر.
qs-waiting-to-unknown = اتصال منسي
qs-waiting-state-waiting = في الانتظار
qs-waiting-state-delivered = نُسخ { $arrived } من { $total } وتم التحقق
qs-waiting-state-needs = يحتاج إلى إجراء: { $count }
qs-waiting-state-declined = رُفض في { $date }
qs-waiting-state-failed = لم يُنسخ: { $count }
qs-waiting-renew-label = مدة الانتظار من الآن
qs-waiting-renew = جدّد
qs-waiting-cancel-title = هل تلغي هذا الإرسال المنتظر؟
qs-waiting-cancel-body = يحذف PCTwin نسخته، ولن يُرسل شيء آخر. تحتفظ الحواسيب التي لديها الملفات بها. تبقى ملفاتك كما هي.
qs-waiting-keep = أبقه

# Quick Send more
qs-session-add-files = إضافة ملفات
qs-session-send-to = إرسال إلى { $name }
qs-session-send-back = إرسال ملفات في المقابل
qs-session-end = إنهاء
qs-session-ended = انتهى هذا الإرسال. لن يصل شيء آخر.
qs-sends-waiting = ينتظر دوره
qs-sends-sending = جارٍ الإرسال
qs-sends-paused = متوقف مؤقتًا
qs-send-pause = إيقاف مؤقت
qs-send-resume = متابعة
qs-send-cancel = إلغاء

# Open sends: Add files, Send to, several laptops at once, questions that wait, a live file being copied
qs-session-ended-name = { $name }: انتهى هذا الإرسال. لن يصل شيء آخر.
qs-sends-action-name = { $what }: { $name }
qs-sends-title = عمليات الإرسال
qs-sends-intro = يرسل PCTwin إلى ثلاثة حواسيب على الأكثر في الوقت نفسه، وإرسالًا بعد آخر إلى الحاسوب نفسه. ينتظر الباقي دوره.
qs-sends-empty = لا يُرسل شيء الآن.
qs-sends-ended-title = عمليات إرسال انتهت
qs-sends-stopped = أُوقف قبل النهاية.
qs-sends-result-whole = نُسخت وتم التحقق منها
qs-sends-result-partial = لم يُنسخ بعضها
qs-sends-not-sent = لم يُرسل إلى { $name }.
qs-sends-failed = لم يستطع PCTwin تغيير هذا الإرسال. حاول مرة أخرى.
qs-session-offer-waiting = يرسل { $name } إليك ملفات أخرى.
qs-session-offer-show = اختر مكان الحفظ
qs-to-now-several-hint = يجب أن يكون كل حاسوب مشغّلًا وقريبًا. يرسل PCTwin إلى ثلاثة على الأكثر في الوقت نفسه، وينتظر الباقي دوره.
qs-asking-files = يريد { $label } أن يرسل إليك ملفات.
qs-asking-note = أرسل إليك { $label } ملاحظة.
qs-asking-later = يسألك PCTwin عن ذلك عندما تغادر هذه الصفحة.
qs-asking-show = اختر مكان الحفظ
qs-asking-show-note = اعرض الملاحظة
progress-current-open-in = أثناء نسخ PCTwin للملف { $name }، لا يستطيع { $app } حفظ تغييرات فيه. إذا قال { $app } إن الملف قيد الاستخدام، فانتظر حتى ينتهي النسخ.

# 2.6 waiting for free Wi-Fi
fullmove-rebuilding-waiting-wifi = { $minutes ->
        [zero] في انتظار شبكة Wi-Fi مجانية قبل تنزيل التطبيقات: لحظات
        [one] في انتظار شبكة Wi-Fi مجانية قبل تنزيل التطبيقات: دقيقة واحدة أخرى على الأكثر
        [two] في انتظار شبكة Wi-Fi مجانية قبل تنزيل التطبيقات: دقيقتان أخريان على الأكثر
        [few] في انتظار شبكة Wi-Fi مجانية قبل تنزيل التطبيقات: { $minutes } دقائق أخرى على الأكثر
        [many] في انتظار شبكة Wi-Fi مجانية قبل تنزيل التطبيقات: { $minutes } دقيقة أخرى على الأكثر
       *[other] في انتظار شبكة Wi-Fi مجانية قبل تنزيل التطبيقات: { $minutes } دقيقة أخرى على الأكثر
    }
