# PCTwin screens, zh-CN (Simplified Chinese). Machine-translated ahead of time from ../en/main.ftl;
# a string missing here falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = PCTwin首页
language-label = 语言
motion-label = 动画
motion-system = 动画：跟随系统
motion-reduced = 动画：减少
motion-off = 动画：关闭

# The opening (Wordplay); its tagline is the home headline.
opening-label = PCTwin 正在打开
opening-status = 正在打开 PCTwin。
opening-skip = 跳过开场
opening-tagline = 发送文件，或者全部搬过去。

footer-free = 免费。无需账户。
footer-help = 帮助

## F01 Home

welcome-title-line1 = 发送文件，
welcome-title-line2 = 或者全部搬过去。
welcome-body = 把几个文件发送到附近的笔记本电脑，或者把你的文件、应用和设置搬到新的笔记本电脑上。
welcome-start = 开始完整迁移

## F01 Home: the cards, My history, a move to resume, files waiting

home-quick-send-body = 发送或接收文件
home-move-body = 迁移你的设置
home-restore-body = 把它还原回来
home-rescue-body = 保住重要的内容
home-history = 我的记录
home-history-body = 你的迁移及其报告
name-title = 给这台笔记本电脑起个名字。
name-body = 附近的其他笔记本电脑会通过这个名字找到它。之后配对时也可以更改。
name-use = 使用这个名字
home-update-title = 有新版本可用。
home-update-action = 立即更新
home-updating = 正在更新。PCTwin 会关闭并自动重新打开。
home-update-failed = 无法安装此更新。PCTwin 仍照常工作；下次打开时会再次提供更新。
home-update-busy = 请先完成迁移，然后再更新 PCTwin。
home-resume-body = 进度已保存。
home-resume-action = 继续
    .aria-label = 恢复你的迁移
home-waiting =
    { NUMBER($count) ->
       *[other] { $count } 个文件正在等待 { $to }
    }

## Header: this laptop, the language and motion control, Help

header-this-laptop = 你的电脑

## F06 Old or new laptop

role-back = 返回
role-eyebrow = 开始完整迁移
role-title = 这是哪台笔记本电脑？
role-body = 在两台笔记本电脑上都打开PCTwin，然后选择这是哪一台。
role-old-title = 这是我的旧笔记本电脑
role-old-body = 发送我的文件、应用和设置。
role-new-title = 这是我的新笔记本电脑
role-new-body = 从另一台笔记本电脑接收我的配置。
role-note = 在你查看并同意方案之前，什么都不会迁移。迁移期间，你的旧笔记本电脑保持原样。

## Frame parts (footer note, Back, steps)

footer-note-move = 个人迁移免费
footer-note-send = 附近发送免费
frame-back = 返回
frame-step-done = 已完成
frame-step-current = 当前
frame-step-todo = 待完成
frame-step-position = 第{ $number }步，共{ $total }步

## F01 Unfinished move

resume-open = 恢复你的迁移

## C01 How will you connect?

connect-eyebrow = 你的迁移
connect-title = 你想怎样连接？
connect-body = 无论选择哪种方式，步骤都一样。
connect-wifi-title = Wi-Fi
connect-wifi-body = 两台笔记本电脑连接同一个Wi-Fi。
connect-wifi-status = 可用
connect-later = 网线、PCTwin桥接线以及通过外接硬盘迁移，将在以后的版本中提供。
connect-usb-note = 普通USB线不能连接两台笔记本电脑来迁移。
connect-use-wifi = 使用Wi-Fi
connect-unit-mbps = { NUMBER($value, maximumFractionDigits: 0) } MB/s
connect-unit-gb = { NUMBER($value, maximumFractionDigits: 1) } GB

## C02 Put both laptops together

connect-prep-title = 把两台笔记本电脑放在一起。
connect-prep-body = 把它们连接到同一个Wi-Fi，然后在每台电脑上打开PCTwin。
connect-prep-step-1-title = 连接同一个Wi-Fi
connect-prep-step-1-body = 复制文件不需要连接互联网。
connect-prep-step-1-network = 网络：{ $network }。
connect-prep-step-2-title = 允许查找附近的笔记本电脑
connect-prep-step-2-body = 如果Windows或macOS请求权限，请选择“允许”。
connect-prep-step-3-title = 在每台笔记本电脑上选择角色
connect-prep-step-3-body = 一台发送，另一台接收。
connect-prep-note = 访客或公共Wi-Fi可能让笔记本电脑无法发现彼此。
connect-prep-continue = 继续配对

## C06 Meet your other laptop

peer-eyebrow = 你的迁移
peer-title = 这是你的另一台笔记本电脑。
peer-body = 两台笔记本电脑都已连接并准备好。
peer-sending = 发送中
peer-receiving = 接收中
peer-old = 旧笔记本电脑
peer-new = 新笔记本电脑
peer-version-label = PCTwin
peer-version = { $version }
peer-status-update = 需要更新
peer-status-unsupported = 等待中
peer-ready-note = 两台笔记本电脑配对后，PCTwin才会列出你的文件。如果版本不同，请在两台电脑上都更新PCTwin。
peer-version-differs = 这两台笔记本电脑上的PCTwin版本不同。请在两台电脑上都更新PCTwin后继续。
peer-version-old-body = 版本不同：{ $version }。
peer-version-new-body = 两台笔记本电脑需要相同的版本。
peer-update-title = 更新PCTwin
peer-update-body = 让两台笔记本电脑安装相同的版本。
peer-direction-unsupported = 这个版本的PCTwin暂时还不能在这两种系统之间迁移。
peer-continue = 检查连接

## C07 Let’s check the connection

check-eyebrow = 你的迁移
check-title = 我们来检查连接。
check-body = PCTwin会检查速度、空间和电源，以便规划迁移并估算时间。
check-measuring = 正在测量连接……
check-speed-label = 连接速度
check-space-label = 新笔记本电脑上可用
check-link-weak = 信号弱，迁移可能较慢。把笔记本电脑移近Wi-Fi路由器会有帮助。
check-power-title = 电源
check-power-both = 两台笔记本电脑都已连接电源。
check-power-battery = { $label }正在使用电池。请插上电源，以免长时间的迁移被打断。
check-space-title = 存储空间
check-space-body = 新笔记本电脑上有{ $free }可用空间。在迁移开始前，你会看到你的选择是否放得下。
check-internet-title = 互联网
check-internet-on = 已连接。复制文件不需要互联网。
check-internet-off = 未连接。没关系，复制文件不需要互联网。
check-status-ready = 已准备好
check-status-plug-in = 插上电源
check-status-connected = 可用
check-status-not-needed = 不需要
check-status-unknown = 未知
check-speed-unknown = PCTwin这次没能测量连接。文件照样会搬过去，只是没有时间估计。
check-space-unknown = PCTwin无法读取新笔记本电脑上的可用空间。
check-power-unknown = PCTwin无法判断{ $label }是否插上了电源。
check-internet-unknown = PCTwin无法判断这台笔记本电脑是否在线。没关系，复制文件不需要互联网。
check-again = 再次检查
check-estimate-note = 预计时间仅供参考，并非固定承诺。
check-continue = 选择要迁移的内容
check-continue-new = 继续
# Added 23 October 2026: the short checklist both laptops show before the move.
check-before-title = 迁移开始前
check-before-lids-title = 两台电脑的屏幕都保持打开
check-before-lids-body = 合上屏幕可能会让笔记本电脑进入睡眠，迁移就会暂停。
check-before-near-title = 靠近Wi-Fi路由器
check-before-near-body = 迁移期间，让两台笔记本电脑都靠近路由器。

## C09 Waiting for your other laptop

reconnect-title = 正在等待你的另一台笔记本电脑。
reconnect-body = 目前已复制并检查的内容都会保留。
reconnect-card-title = 连接中断，迁移已暂停
reconnect-progress =
    .aria-label = 连接中断，迁移已暂停
reconnect-saved = 目前已复制并检查{ $done }，其余部分稍后继续。
reconnect-last-seen = 上次在线：{ $ago }。
reconnect-status-offline = 离线
reconnect-status-trying = 正在重新连接
reconnect-status-pending = 等待中
reconnect-trying = 正在自动重新连接……请让两台笔记本电脑保持开机、不要休眠，并连接同一个Wi-Fi。
reconnect-expired = 时间太久，无法自动重新连接。请重新配对，PCTwin会检查哪些内容已经复制过。
reconnect-unavailable = 无法连接到{ $label }。请检查它是否已开机、没有休眠、打开了PCTwin，并连接同一个Wi-Fi。
reconnect-try-now = 重试连接
reconnect-pair-again = 重新配对
reconnect-nothing-new = 已经有一段时间没有新内容传来。PCTwin 会继续自动尝试，你也可以停止移动。目前已复制并检查过的内容都会保留。
reconnect-details = 安全地重新连接
reconnect-details-body = 只有这两台笔记本电脑可以重新连接，而且每次都使用新的私密连接。

## C10 The connection has ended (dialog)

ended-title = 连接已结束。
ended-body = 你的迁移已保存。旧的配对码已失效。
ended-kept-title = 已保留
ended-kept-body = 你的迁移记录，以及所有已复制并检查的内容。
ended-not-kept-title = 未保留
ended-not-kept-body = 无需重新配对即可继续的权限。
ended-pair-again = 重新配对
ended-pair-again-body = 重新连接后，PCTwin会检查哪些内容已经复制过。
ended-see-moved = 打开已保存的迁移
ended-see-moved-body = 查看目前已复制并检查的内容。
ended-home = 返回首页

## Pairing, shared

pair-back = 返回
pair-try-again = 重试
pair-more-details = 更多详情
pair-heads-up-title = 你的笔记本电脑可能会问你一个问题。
pair-heads-up-local-network = 你的Mac会询问PCTwin能否查找你本地网络上的设备。请选择“允许”，这样两台笔记本电脑才能找到彼此。
pair-heads-up-firewall = Windows可能会询问PCTwin能否接受连接。请选择“允许”，这样你的新笔记本电脑才能连接到这台电脑。
pair-heads-up-continue = 继续

## C03 Old laptop: show the code

pair-old-eyebrow = 旧笔记本电脑
pair-old-title = 我们来连接你的笔记本电脑。
pair-old-body = 在你的新笔记本电脑上输入下面的配对码。
pair-old-next = 接下来，你要核对两台笔记本电脑上显示的同一个数字。
pair-old-network = 让两台笔记本电脑保持连接同一个Wi-Fi。
pair-old-waiting = 正在等待你的新笔记本电脑……
pair-old-starting = 正在准备配对码……
pair-old-label-title = 这台笔记本电脑的名字
pair-old-label-hint = 你的新笔记本电脑会通过这个名称找到这台电脑。
pair-old-shuffle = 换一个名称
pair-old-rename = 输入名称
pair-old-rename-label = 这台笔记本电脑的名称
pair-old-rename-note = 这个Wi-Fi上的任何人都能看到这个名称。
pair-old-rename-save = 使用这个名称
pair-old-rename-cancel = 取消
pair-old-code-meta =
    { $seconds ->
       *[other] 每分钟更换一次。{ $seconds }秒后出现新配对码。
    }
pair-old-steps-title = 在你的新笔记本电脑上
pair-old-step-1 = 打开PCTwin，选择“我的新笔记本电脑”。
pair-old-step-2 = 如果出现列表，请选择这台笔记本电脑的名称。
pair-old-step-3 = 输入这里显示的六位配对码。
pair-old-wait-title = 新配对码马上就来。
pair-old-wait-body =
    { $seconds ->
       *[other] 这次尝试没有成功。{ $seconds }秒后会出现新配对码。
    }
pair-old-locked-title = 这个网络上有人尝试了错误的配对码。
pair-old-locked-body = 已尝试五次错误的配对码，为保护这台笔记本电脑的安全，配对已停止。没有共享任何内容。如果是你自己，请选择“重新开始”获取新的配对码。
pair-old-locked-hotspot = 如果不知道是谁，请改为把两台笔记本电脑都连接到你手机的热点，然后选择“重新开始”。
pair-old-start-again = 重新开始
pair-old-timed-out = 没有及时选择数字，这是新的配对码。
pair-old-declined = 数字不匹配，因此没有共享任何内容。这是新的配对码。
pair-old-left = 在选择数字之前，新笔记本电脑停止了配对，因此没有共享任何内容。这是新的配对码。
pair-network-title = 选择你的Wi-Fi。
pair-network-hint = 选择你的新笔记本电脑所连接的网络。

## C05 Old laptop: match the number

pair-choose-eyebrow = 确认连接
pair-choose-title = 选择相同的数字。
pair-choose-body = 看看你的新笔记本电脑，选择它显示的数字。
pair-choose-number = 数字{ $number }
pair-choose-confirm = 确认这两台笔记本电脑
pair-choose-renewed = 已显示新的数字。请选择你的新笔记本电脑上显示的那个。
pair-choose-none = 都不匹配
pair-choose-note = 如果新笔记本电脑没有显示数字，或者这些数字都不匹配，请选择“都不匹配”。
pair-choose-safe = 目前还没有共享任何内容。
pair-choose-checking = 正在检查……

## C04 New laptop: find the old laptop and enter the code

pair-new-eyebrow = 新笔记本电脑
pair-new-searching-title = 正在搜索你的旧笔记本电脑……
pair-new-searching-body = 请在旧笔记本电脑上保持PCTwin打开，并与这台电脑连接同一个Wi-Fi。
pair-new-search-failed-title = 无法查找你的旧笔记本电脑。
pair-new-list-title = 哪一台是你的旧笔记本电脑？
pair-new-list-body = 选择你的旧笔记本电脑屏幕上显示的名称。
pair-new-same-label = 附近另一台笔记本电脑也叫这个名称。请查看旧笔记本电脑的屏幕。
pair-new-needs-update = 这台笔记本电脑上的 PCTwin 版本较旧。请在它上面更新 PCTwin，然后再次搜索。
pair-new-none-title = 没有找到旧笔记本电脑。
pair-new-none-body = 请检查旧笔记本电脑上的PCTwin是否已打开并显示配对码，以及两台电脑是否连接同一个Wi-Fi。然后重新搜索。
pair-new-search-again = 重新搜索
pair-old-same-role = 附近另一台笔记本电脑也被设为旧电脑。两台中必须有一台是新电脑。
pair-old-switch = 这是新电脑
pair-new-none-same-role = 另一台电脑是否也选择了新电脑？那就在那台上选择旧电脑，或者切换这一台。
pair-new-switch = 这是旧电脑
pair-new-code-title = 输入配对码。
pair-new-code-body = 输入{ $label }上显示的配对码。
pair-new-code-label = 六位配对码
pair-new-code-hint = 看你的旧笔记本电脑屏幕上的配对码。它每分钟更换一次。
pair-new-connect = 连接
pair-new-connecting = 正在连接……
pair-new-other-laptop = 选择另一台笔记本电脑
pair-new-number-eyebrow = 确认连接
pair-new-number-title = 在你的旧笔记本电脑上选择这个数字。
pair-new-number-tile = 你的数字
pair-new-number-label = 你的数字是{ $number }
pair-new-number-body = 你的旧笔记本电脑会显示三个数字，请在那里选择这一个。
pair-new-number-waiting = 正在等待你的旧笔记本电脑……

## Paired (both laptops)

pair-done-eyebrow = 一切就绪
pair-done-title = 你的笔记本电脑已配对。
pair-done-body = 两台笔记本电脑已连接。接下来，旧笔记本电脑会查找你的文件，你在新笔记本电脑上批准要移动的内容。你批准之前，什么都不会移动。
pair-done-continue = 继续

## Pairing problems (each says what happened, then what to do)

pairing-code-invalid = 配对码需要六位数字。请查看旧笔记本电脑的屏幕，然后重新输入。
pairing-code-wrong = 配对码不正确。请查看旧笔记本电脑的屏幕，然后再次尝试。
pairing-code-expired = 配对码已更换。请输入旧笔记本电脑屏幕上的新配对码。
pairing-busy = 旧笔记本电脑正在与另一台电脑配对。请稍等片刻，然后再次尝试。
pairing-wait = 旧笔记本电脑正在准备新的配对码。请等待它准备好，然后输入新配对码。
pairing-locked = 错误尝试次数过多，配对已停止。请在旧笔记本电脑上选择“重新开始”。
pairing-unreachable = 这台笔记本电脑无法连接到旧笔记本电脑。请检查两台电脑是否连接同一个Wi-Fi，然后再次尝试。
pairing-wrong-number = 这个数字与新笔记本电脑上的不匹配。没有共享任何内容。请在新电脑上输入新的配对码，再次尝试。
pairing-no-match = 旧笔记本电脑上的数字不匹配。没有共享任何内容。请输入旧电脑屏幕上的新配对码。
pairing-timed-out = 没有及时选择数字。请使用旧笔记本电脑上的新配对码再次尝试。
pairing-connection-lost = 两台笔记本电脑之间的连接已断开。没有共享任何内容。请再次尝试。
pairing-connection-lost-new = 与旧笔记本电脑的连接已断开。没有共享任何内容。
pairing-different-version = 两台笔记本电脑上的PCTwin版本不同。请在两台电脑上都更新PCTwin，然后再次尝试。
pairing-name-invalid = 这个名称不能使用。请换一个更短的名称，只用字母和数字。
pairing-no-network = 这台笔记本电脑没有连接Wi-Fi或有线网络。请连接网络，然后再次尝试。
pairing-choose-network = 这台笔记本电脑连接了多个网络。请选择你的新笔记本电脑所连接的那个：
pairing-local-network-refused = 你的Mac可能不允许PCTwin查找这个网络上的笔记本电脑。如果Mac刚刚询问过你，请选择“允许”，然后重试。否则，请打开“系统设置”，选择“隐私与安全性”，再选择“本地网络”，打开PCTwin，然后重试。
pairing-local-network-refused-details = VPN、防火墙或过滤软件也可能让笔记本电脑无法找到彼此。如果有其中之一正在运行，请在配对期间关闭它，然后重试。
pairing-failed = 这次配对没有成功。没有共享任何内容。请再次尝试。
pairing-failed-new = 这次配对没有成功。没有共享任何内容。请输入旧笔记本电脑屏幕上的新代码。

## M06 Match people to their profiles (Who’s moving?)

people-eyebrow = 你的迁移
people-title = 为每个人匹配对应的账户。
people-body = 选择每个人的文件放到新笔记本电脑上的哪里。
people-col-old = 旧笔记本电脑
people-col-new = 新笔记本电脑
people-col-status = 状态
people-you = 你
people-select =
    .aria-label = { $name }在新笔记本电脑上的账户
people-option-existing = { $account } / 新笔记本电脑
people-option-new = 创建新账户
people-option-leave-out = 不包括
people-option-choose = 请选择……
people-new-account-hint = 新笔记本电脑会在创建账户之前请求权限。
people-suggested = 因为名称相同而推荐。请确认是同一个人。
people-suggested-use = 使用{ $account }
people-status-ready = 已确认
people-status-waiting = 需要对方同意
people-status-left-out = 未包括
people-status-choose = 需要你选择
people-status-changed = 已由你更改
people-waiting-body = { $name }需要在旧笔记本电脑上登录并同意，其文件才能迁移。在此之前，这些文件不会被改动。
people-just-me = 在旧笔记本电脑上只找到了你自己的账户。
people-changed-note = 你更改了某人文件的去向。继续之前请检查一下。
people-shared-title = 共享项目
people-shared-body = 属于旧笔记本电脑上所有人的共享文件夹和公共文档。
people-shared-select =
    .aria-label = 共享项目的去向
people-shared-option-shared = 新笔记本电脑上的共享文件夹
people-shared-option-person = 和{ $name }的文件放在一起
people-shared-option-leave-out = 不包括
people-note = PCTwin绝不会更改任何人的密码，也不会绕过他人的登录。
people-list-label = 旧笔记本电脑上的人
people-todo =
    { $count ->
        [0] 每个可以迁移的人都有了去处。
       *[other] 还有{ $count }个人需要你选择。
    }
people-waiting-count =
    { $count ->
       *[other] 有{ $count }个人正在等待同意。你可以继续；在对方同意之前，其文件不会被改动。
    }
people-continue = 继续

## M26 Before you leave this laptop (things to do on the old laptop)

before-eyebrow = 你的迁移
before-title = 在离开这台笔记本电脑之前。
before-body = 趁旧笔记本电脑还在手边，检查一下这些事项。
before-licences-title = 应用许可证
before-licences-body = 有些应用需要你自己转移许可证。
before-passwords-title = 账户找回
before-passwords-body = 确保你仍然能登录你的账户，并能打开任何上了锁的硬盘。
before-codes-title = 把登录验证码放在手边
before-codes-body = 如果某个账户会给你发验证码，或者用应用确认是你本人，请把那部手机或应用放在身边。
before-online-title = 未保存的工作
before-online-body = 保存所有草稿，并检查在线存储的文件是否是最新的。
before-done =
    .aria-label = 我已完成：{ $item }
before-note = 不要在这里输入密码或恢复密钥。完成这份列表并不代表旧笔记本电脑可以安全清空。
before-empty = 迁移之前，旧笔记本电脑上没有需要处理的事项。
before-continue = 返回迁移方案

## S11 Get help with this task (a short problem code)

help-eyebrow = 帮助和隐私
help-title = 获取这项任务的帮助。
help-body = 分享一个简短的问题码，让别人知道出了什么问题。请先查看它包含哪些内容。
help-making = 正在生成问题码……
help-code-title = 问题码
help-code =
    .aria-label = 问题码{ $code }
help-contains-title = 包含的内容
help-contains-problem = 问题的类型，以及任务进行到了哪一步。
help-contains-step = 问题发生在迁移的哪一步。
help-contains-versions = PCTwin 的版本，以及这台笔记本电脑运行的系统，例如 Windows 或 macOS。
help-never-title = 绝不包含的内容
help-chip-included = 包含
help-chip-excluded = 绝不包含
help-never-names = 你或任何其他人的名字。
help-never-files = 你的文件内容，或任何密码。
help-never-network = 文件或文件夹名称、文件存放的地方，或Wi-Fi名称。
help-note = 这个问题码不会让任何人访问你的笔记本电脑。
help-copy = 复制问题码
help-copied = 问题码已复制。
help-share = 分享
help-share-hint = 通过WhatsApp或任何应用发送。
help-share-text = 我的PCTwin问题码是{ $code }。
help-failed = 无法生成问题码。请再次尝试。如果还是不行，请改用你自己的话描述问题。
help-try-again = 重试
help-copy-failed = 无法复制问题码。请选中上面的问题码，自己复制。
help-home-eyebrow = 帮助
help-home-title = 帮你完成眼前的任务。
help-home-body = 针对最常见障碍的简短指南，无需联网即可使用。
help-home-more = 在 getpctwin.com 获取更多帮助
help-guide-find-title = 两台笔记本电脑找不到对方
help-guide-find-body = 让两台笔记本电脑连接同一个 Wi-Fi，不要用访客网络。配对时关闭所有 VPN。如果仍然不行，打开手机热点，让两台电脑都连上它。
help-guide-mac-network-title = Mac 询问本地网络权限
help-guide-mac-network-body = 选择“允许”，PCTwin 才能找到另一台笔记本电脑。如果选了“不允许”，请打开“系统设置”，再打开“隐私与安全性”，然后打开“本地网络”，并打开 PCTwin。
help-guide-firewall-title = Windows 询问防火墙权限
help-guide-firewall-body = 在专用网络（例如家里的 Wi-Fi）上允许 PCTwin。PCTwin 只在你自己的网络上与另一台笔记本电脑通信。
help-guide-stopped-title = 移动中途停止了
help-guide-stopped-body = 已复制并检查的内容不会丢失，旧笔记本电脑也没有被更改。重新配对两台电脑即可：新电脑上已有的文件会被识别，不会重复复制。
help-guide-others-title = 移动其他人的内容
help-guide-others-body = 在旧笔记本电脑的“谁要搬过去？”中勾选他们。每台电脑上由管理员批准一次。你永远看不到他们的文件。新账户的临时密码只会在电脑自己的窗口中显示一次；他们首次登录时设置自己的密码。
help-guide-code-title = 向别人求助
help-guide-code-body = 在任何问题页面上选择“获取帮助”。PCTwin 会生成一个简短代码说明出了什么问题，不含姓名、文件或密码，你可以通过 WhatsApp 或任何应用发送。

## Label words (the name each laptop shows while pairing)

label-colour-red = 红色
label-colour-orange = 橙色
label-colour-yellow = 黄色
label-colour-green = 绿色
label-colour-blue = 蓝色
label-colour-purple = 紫色
label-colour-pink = 粉色
label-colour-brown = 棕色
label-colour-black = 黑色
label-colour-white = 白色
label-colour-grey = 灰色
label-colour-gold = 金色

label-animal-ant = 蚂蚁
label-animal-badger = 獾
label-animal-bat = 蝙蝠
label-animal-bear = 熊
label-animal-bee = 蜜蜂
label-animal-beetle = 甲虫
label-animal-bison = 野牛
label-animal-butterfly = 蝴蝶
label-animal-camel = 骆驼
label-animal-cat = 猫
label-animal-cheetah = 猎豹
label-animal-chicken = 母鸡
label-animal-crab = 螃蟹
label-animal-crane = 仙鹤
label-animal-crocodile = 鳄鱼
label-animal-crow = 乌鸦
label-animal-deer = 鹿
label-animal-dolphin = 海豚
label-animal-donkey = 驴
label-animal-dove = 鸽子
label-animal-dragonfly = 蜻蜓
label-animal-duck = 鸭子
label-animal-eagle = 老鹰
label-animal-elephant = 大象
label-animal-falcon = 猎鹰
label-animal-flamingo = 火烈鸟
label-animal-fox = 狐狸
label-animal-frog = 青蛙
label-animal-gazelle = 羚羊
label-animal-swordfish = 剑鱼
label-animal-giraffe = 长颈鹿
label-animal-goat = 山羊
label-animal-goose = 鹅
label-animal-gorilla = 大猩猩
label-animal-hamster = 仓鼠
label-animal-hawk = 苍鹰
label-animal-hedgehog = 刺猬
label-animal-armadillo = 犰狳
label-animal-hippo = 河马
label-animal-horse = 马
label-animal-hummingbird = 蜂鸟
label-animal-jaguar = 美洲豹
label-animal-jellyfish = 水母
label-animal-kangaroo = 袋鼠
label-animal-kingfisher = 翠鸟
label-animal-koala = 考拉
label-animal-ladybird = 瓢虫
label-animal-lemur = 狐猴
label-animal-leopard = 豹子
label-animal-lion = 狮子
label-animal-lizard = 蜥蜴
label-animal-llama = 羊驼
label-animal-lobster = 龙虾
label-animal-lynx = 猞猁
label-animal-macaw = 金刚鹦鹉
label-animal-meerkat = 狐獴
label-animal-mole = 鼹鼠
label-animal-moose = 驼鹿
label-animal-pufferfish = 河豚
label-animal-mouse = 老鼠
label-animal-octopus = 章鱼
label-animal-ostrich = 鸵鸟
label-animal-otter = 水獭
label-animal-owl = 猫头鹰
label-animal-panda = 熊猫
label-animal-kiwi = 几维鸟
label-animal-parrot = 鹦鹉
label-animal-peacock = 孔雀
label-animal-pelican = 鹈鹕
label-animal-penguin = 企鹅
label-animal-puffin = 海鹦
label-animal-rabbit = 兔子
label-animal-raccoon = 浣熊
label-animal-rhino = 犀牛
label-animal-robin = 知更鸟
label-animal-salmon = 三文鱼
label-animal-seahorse = 海马
label-animal-seal = 海豹
label-animal-shark = 鲨鱼
label-animal-sheep = 绵羊
label-animal-sloth = 树懒
label-animal-snail = 蜗牛
label-animal-sparrow = 麻雀
label-animal-squirrel = 松鼠
label-animal-starfish = 海星
label-animal-stork = 白鹳
label-animal-swan = 天鹅
label-animal-tiger = 老虎
label-animal-snake = 蛇
label-animal-toucan = 巨嘴鸟
label-animal-turtle = 海龟
label-animal-walrus = 海象
label-animal-whale = 鲸鱼
label-animal-wolf = 狼
label-animal-woodpecker = 啄木鸟
label-animal-yak = 牦牛
label-animal-zebra = 斑马
label-animal-dog = 狗
label-animal-cow = 奶牛
label-animal-beaver = 河狸

label-picked = { $colour }{ $animal }
label-unnamed = 未命名的笔记本电脑

## M02 Finding your files

scan-eyebrow = 你的迁移
scan-title = 正在查找你熟悉的配置。
scan-body = PCTwin找到的内容会被保存，所以搜索停止后也能继续。
scan-progress =
    .aria-label = 正在搜索你选择的地方
scan-found =
    { NUMBER($count) ->
       *[other] 目前已找到{ $count }个文件。
    }
scan-step-usual-title = 文件和文件夹
scan-step-usual-body = 已搜索PCTwin可以读取的地方。
scan-step-other-title = 其他文件夹和硬盘
scan-step-other-body = 你在常用地方以外使用的文件夹和硬盘。
scan-step-protected-title = 受保护的地方
scan-step-protected-body =
    { NUMBER($count) ->
       *[other] 还有{ $count }个地方需要你的权限。
    }
scan-step-done = 已完成
scan-step-current = 当前
scan-step-todo = 待完成
scan-unreadable-note = PCTwin无法读取的地方会留在列表中，不算作已搜索。
scan-status-not-searched = 未搜索
scan-paused = 搜索已暂停。目前找到的所有内容都已保存。
scan-resumed = 搜索已从停止的地方继续。
scan-partial =
    { NUMBER($count) ->
       *[other] 还有{ $count }个受保护的地方尚未搜索。
    }
scan-allow = 允许访问
scan-pause = 暂停
scan-resume = 继续搜索
scan-continue = 查看找到的内容

## M03 Everything you chose, in view

overview-eyebrow = 你的迁移
overview-title = 你选择的所有内容，一目了然。
overview-body = 在PCTwin制定方案之前，检查它找到的内容。
overview-size-label = 已选文件
overview-items-label =
    { NUMBER($count) ->
       *[other] 个文件
    }
overview-attention-label =
    { NUMBER($count) ->
       *[other] 需要你留意
    }
overview-files-title = 文件和文件夹
overview-files-body = 你选择的文档、照片和其他文件夹。
overview-dest-title = 去向
overview-dest-body = 每个文件夹在新笔记本电脑上存放的地方。
overview-people-title = 人员
overview-people-body = 为每个人选择新笔记本电脑上的账户。
overview-review = 查看
overview-partial =
    { NUMBER($count) ->
       *[other] 还有{ $count }个受保护的地方尚未搜索。
    }
overview-allow = 允许访问
overview-continue = 继续

## M04 Keep what matters to you

files-eyebrow = 你的迁移
files-title = 保留对你重要的内容。
files-body = 选择文件夹、其他硬盘以及任何其他地方。
files-search-label = 查找文件夹
files-search-placeholder = 搜索文件夹
files-select =
    .aria-label = 迁移{ $name }
files-folder-detail =
    { NUMBER($count) ->
       *[other] { $size } · { $count }个文件
    }
files-status-protected = 需要权限
files-protected-body = PCTwin需要你的权限才能读取这个文件夹。在你允许之前，其中的文件不计入统计。
files-allow = 允许访问
files-status-added = 由你添加
files-add = 添加其他文件夹
files-remove =
    .aria-label = 移除{ $name }
files-no-limit = 免费迁移对文件大小和文件夹数量都没有限制。
files-total =
    { NUMBER($count) ->
       *[other] 已选择{ $count }个文件夹，共{ $size }
    }
files-none = 至少选择一个文件夹才能继续。
files-continue = 选择目标位置

## M05 Give every file a home

dest-eyebrow = 你的迁移
dest-title = 给每个文件安个家。
dest-body = 选择每个文件夹放到新笔记本电脑上的哪里。
dest-col-chosen = 你选择的内容
dest-col-place = 去向
dest-col-placement = 硬盘
dest-placement-local = 内置
dest-placement-external = 外接
dest-col-status = 状态
dest-select =
    .aria-label = { $name }的目标位置
dest-option-folder = 新笔记本电脑 / { $folder }
dest-option-drive = 外接硬盘 / { $drive }
dest-option-choose = 选择文件夹
dest-default-label = 默认位置
dest-status-ready = 已准备好
dest-status-missing = 硬盘未连接
dest-status-space = 空间不足
dest-all-ready = 每个文件夹在新笔记本电脑上都有了存放的地方。
dest-missing = { $drive }没有连接到新笔记本电脑。请连接它，或为这些文件选择其他地方。
dest-space = { $place }有{ $free }可用空间，但这些文件需要{ $needed }。请在那里腾出空间、选择其他地方，或减少要迁移的文件。
dest-change = 更改
dest-continue = 查看方案

## M23 Approve the move you want

plan-eyebrow = 你的迁移
plan-title = 同意你想要的迁移方案。
plan-body = PCTwin只做这个方案里写的事。
plan-what-title = 你选择的内容
plan-what-body = 来自你选择的文件夹的{ $size }文件，将放到你挑选的地方。
plan-where-body =
    { NUMBER($count) ->
       *[other] 放到新笔记本电脑上的{ $count }个地方。
    }
plan-hand-title = 开始之前
plan-hand-body =
    { NUMBER($count) ->
       *[other] { $count }个步骤。它们会列在最后的报告中。
    }
plan-hand-none = 没有（就PCTwin目前所知）。
plan-old-safe = 你的旧笔记本电脑保持原样。文件只会被复制，绝不会从旧电脑上移走。
plan-edit = 更改方案
plan-edit-note = 如果你更改方案，需要在任何新内容开始之前重新同意。
plan-approve = 我同意这个方案，包括需要我自己完成的事项。
plan-not-approved = 你还没有同意这个方案，所以不会迁移任何内容。
plan-changed = 你更改了方案，所以在迁移任何新内容之前，请重新同意。
plan-start = 开始迁移

## Plan screens: shared words and the lines the copy deck did not cover

plan-back = 返回
plan-number = { NUMBER($n) }
plan-size-b = { NUMBER($n) }字节
plan-size-kb = { NUMBER($n) } KB
plan-size-mb = { NUMBER($n) } MB
plan-size-gb = { NUMBER($n) } GB
plan-size-tb = { NUMBER($n) } TB
plan-size-at-least = 至少{ $size }
plan-size-unknown = 大小未知
plan-role-home = 主文件夹
plan-role-desktop = 桌面
plan-role-documents = 文档
plan-role-downloads = 下载
plan-role-pictures = 图片
plan-role-music = 音乐
plan-role-videos = 视频
plan-role-public = 公共文件夹
plan-role-app-data = 应用数据
plan-role-other = 其他文件夹
scan-places = 已搜索{ $done }/{ $total }个地方
scan-card-title = 正在搜索你选择的地方
scan-pausing = 处理完当前文件夹后暂停。目前找到的所有内容都已保存。
overview-row-attention = 需要留意
files-status-work = 工作或学校
files-work-body = 这个文件夹由你的工作单位或学校管理，所以除非你勾选它，否则不会包括在内。
files-online-only =
    { NUMBER($count) ->
       *[other] 有{ $count }个文件仅在线存储，所以不会被复制。
    }
files-no-match = 没有与这个名称匹配的文件夹。
dest-status-none = 选择地方
dest-status-readonly = 无法写入
dest-status-format = 文件太大
dest-none = 请为{ $name }在新笔记本电脑上选择一个存放的地方。
dest-readonly = { $place }是只读的，无法复制任何内容到上面。请选择其他地方。
dest-format = 这些文件夹中有一个文件太大，按{ $place }目前的设置方式放不下。请选择其他地方。
plan-place = { $drive }上的{ $folder }
plan-group-to = 放到{ $place }
plan-group-move = 移动 { $folder }
plan-place-change = 选择其他位置
plan-place-usual = 使用常用位置
plan-place-free = 那里还有 { $size } 可用空间
plan-place-no-room = 那里的可用空间不够。请选择其他位置，或取消勾选此文件夹。
plan-place-no-safe-name = PCTwin 无法在该驱动器上安全地保存文件。请选择其他位置。
plan-place-cannot-open = PCTwin 无法打开该文件夹。请选择其他位置。
plan-place-locked = 该硬盘已锁定。请先解锁，然后重新选择它。
plan-version = 方案版本{ $revision }
plan-details = 更多详情
plan-preview-title = 开始之前
plan-preview-fits = 这台笔记本电脑有足够的可用空间。
plan-preview-fits-about = 这台笔记本电脑应该有足够的可用空间。其中为 PCTwin 安装的应用及其数据计入了约 { $extra }。
# Added 23 October 2026: what moves first, from the move's own order. $places is a list of folder names.
plan-preview-first = 我们会先迁移这些，让你的新笔记本电脑更快可以使用：{ $places }。
plan-preview-short = 这台笔记本电脑还需要 { $size } 可用空间。请在开始前释放一些空间，否则硬盘满时移动会停止。
plan-preview-short-about = 这台笔记本电脑还需要约 { $size } 可用空间，其中为 PCTwin 安装的应用及其数据计入了约 { $extra }。请在开始前释放一些空间，否则硬盘满时移动会停止。
plan-preview-space-unknown = PCTwin 无法读取这台笔记本电脑的可用空间。
plan-preview-synced = 在这台笔记本电脑上，{ $folder } 保存在 { $service } 中。移到那里的文件会上传到 { $service }，并占用你在那里的空间。
plan-preview-time-one =
    { NUMBER($minutes) ->
       *[other] 大约 { $minutes } 分钟。
    }
plan-preview-time = 大约 { $low } 到 { $high } 分钟。
plan-preview-renamed =
    { NUMBER($count) ->
       *[other] 有 { $count } 个文件的名称会略有不同，因为这台笔记本电脑不允许使用原名称。报告会逐一列出。
    }
plan-preview-too-big =
    { NUMBER($count) ->
       *[other] 有 { $count } 个文件对这台笔记本电脑的硬盘来说太大，不会被复制。
    }
plan-preview-refused =
    { NUMBER($count) ->
       *[other] 有 { $count } 个文件的名称或文件夹不被这台笔记本电脑允许，不会被复制。
    }
plan-preview-runs-at-sign-in =
    { NUMBER($count) ->
       *[other] 有 { $count } 个文件会在有人登录这台笔记本电脑时自动运行。请确认这是你预期的。
    }
plan-reference = 参考编号{ $reference }
plan-hand-cloud-only =
    { NUMBER($count) ->
       *[other] 有{ $count }个文件仅在线存储。PCTwin不会下载它们。请先在旧笔记本电脑上让它们可用，或者跳过它们。
    }
plan-hand-unreadable =
    { NUMBER($count) ->
       *[other] 有{ $count }个项目无法打开，所以不会迁移。请允许访问，或者跳过它们。
    }
plan-hand-work-managed =
    { NUMBER($count) ->
       *[other] 有{ $count }个项目由你的工作单位或学校管理，已被排除。勾选所在的文件夹即可包括它们。
    }
plan-hand-too-many-files =
    { NUMBER($count) ->
       *[other] 有{ $count }个文件夹里的文件太多，一次搬家装不下，所以不会搬过去。这次搬家后，请用快速发送单独发送每个文件夹，或用 USB 闪存盘复制。
    }
plan-blocked-someone-unmapped = 旧笔记本电脑上有人还没有选择新电脑上的账户。请选择每个人的文件放到哪里，然后再回来。
plan-blocked-account-used-twice = 有两个人会被放进新笔记本电脑上的同一个账户。请为每个人分配各自的账户。
plan-blocked-invalid = 这个方案目前无法同意。请返回检查你的选择。
plan-blocked-fix = 选择账户

## Tray menu

tray-show = 显示PCTwin
tray-quit = 退出PCTwin

## F01 secondary actions and their coming-soon pages

welcome-restore = 还原配置
welcome-rescue = 救援笔记本电脑
welcome-quick-send = 快速发送

soon-eyebrow = 即将推出
soon-restore-title = 还原配置
soon-restore-body = 用你在迁移后保存的配置，重建一台笔记本电脑。这项功能将在以后的版本中推出。
soon-rescue-title = 救援笔记本电脑
soon-rescue-body = 把无法启动的笔记本电脑的硬盘连接到一台正常的电脑上，找回其中的文件。这项功能将在以后的版本中推出。
soon-move-title = 完整迁移
soon-move-body = 把旧笔记本电脑上的一切搬到新电脑上：文件、应用和设置。这项功能将在以后的版本中推出。现在，你可以先用快速发送来发送文件。

## X01 Move progress, compact

progress-eyebrow = 你的迁移
progress-title = 正在帮你安顿下来。
progress-title-old = 正在把你的东西发送到新的笔记本电脑。
progress-title-paused = 迁移已暂停
progress-paired = 已配对
progress-percent =
    .aria-label = 迁移进度
progress-amount = { $done } / { $total }
progress-time-left = 大约还剩{ $range }
progress-time-measuring = 正在估算剩余时间……
progress-speed = { $speed }（实测）
progress-count-needs-action =
    { NUMBER($count) ->
       *[other] { $count }个项目需要你处理
    }
progress-current-copying = 正在复制：{ $name }
progress-current-checking = 正在检查：{ $name }
progress-needs-action =
    { NUMBER($count) ->
       *[other] { $count }个项目需要你处理。其余部分继续迁移。
    }
progress-see-needs-action = 查看需要你处理的项目
progress-interrupted = 与另一台笔记本电脑的连接已断开。PCTwin正在尝试重新连接。已复制并检查的内容都不会丢失。
progress-low-battery = 这台笔记本电脑电量不足。请接上电源，以免移动中断。
progress-battery-paused = 这台笔记本电脑电量快用完了，所以移动已安全暂停。接上电源后继续即可。不会丢失任何内容。
progress-old-safe = 你的旧笔记本电脑保持原样。
progress-pause = 暂停迁移
progress-compact = 精简显示
progress-expand = 查看全部迁移内容
progress-stop = 停止迁移

## X02 Move progress, expanded

progress-all-title = 查看全部迁移内容。
progress-all-body = 每个项目都会显示它来自哪里，以及当前进展。
progress-metric-overall = 总进度
progress-metric-committed = 已复制并检查
progress-metric-estimate = 预计
progress-col-item = 项目
progress-col-from = 来源
progress-col-status = 状态
progress-state-waiting = 等待中
progress-state-copying = 正在复制
progress-state-verified = 已检查
progress-state-checking = 正在检查
progress-state-paused = 已暂停
progress-checked-note = 项目只有在检查之后才算已复制。
# Added 23 October 2026: T14, one tip under the progress bar, about every 12 seconds.
tips-label = 提示
tips-pause = 暂停提示
tips-resume = 显示更多提示
tips-next = 下一条提示
tip-battery = 这台笔记本电脑正在使用电池。如果可以，请插上电源，以免长时间的迁移被打断。
tip-closer = 连接较慢。把笔记本电脑移近Wi-Fi路由器可以加快速度。
tip-lids = 请保持这台笔记本电脑的屏幕打开。合上屏幕可能会让它进入睡眠，迁移就会暂停。
tip-keep-using = 你可以继续使用这台笔记本电脑。PCTwin只读取它上面的内容。
tip-pause = 你可以随时暂停迁移，稍后再继续。
tip-power-cut = 如果停电，请打开两台笔记本电脑并打开PCTwin。它会从中断的地方继续，已复制并检查的内容都不会丢失。

## F09 Mini panel

mini-title = 你的迁移正在附近进行。
mini-title-paused = 迁移已暂停。
mini-what = 正在迁移你选择的文件
mini-progress =
    .aria-label = 迁移进度
mini-pause = 暂停
mini-resume = 恢复
mini-open = 打开迁移
mini-title-send = 你的文件正在发送中。
mini-what-send = 正在发送选择的文件
mini-open-send = 打开发送
mini-progress-send =
    .aria-label = 发送进度

## X05 Paused at a safe point

pause-title = 已在安全点暂停。
pause-title-finishing = 正在暂停到安全点……
pause-body = 已复制并检查的内容都会保留，其余的会等待。
pause-finishing = 先完成{ $name }，以免它只复制了一半。
pause-kept-title = 文件
pause-kept-body = 目前已复制并检查的内容都会保留。
pause-waiting-title = 等待中
pause-waiting-body =
    { NUMBER($count) ->
       *[other] 还有{ $count }个项目要复制。没有丢失任何内容。
    }
pause-status-kept = 已暂停
pause-status-waiting = 等待中
pause-note = 暂停不会撤销任何操作。你的旧笔记本电脑保持原样。
pause-resume = 恢复已保存的迁移

## X10 Stop the remaining move? (danger dialog)

cancel-title = 停止剩余的迁移？
cancel-body = 停止后，已复制并检查的内容都会保留。这不会撤销迁移。
cancel-copied-title = 已复制的文件
cancel-copied-body = 保留在新笔记本电脑上。
cancel-rest-title = 剩余的迁移
cancel-rest-body = 不会开始任何新内容。尚未复制的文件只保留在旧笔记本电脑上。
cancel-partial-title = 只复制了一部分的文件
cancel-partial-body = 绝不会被当作完整文件保留。
cancel-old-title = 旧笔记本电脑
cancel-old-body = 保持原样。
cancel-ack = 我明白停止不会撤销已复制的内容
cancel-stop = 停止并查看报告
cancel-keep = 继续迁移

## R01 Welcome to your next laptop (outcome)

outcome-eyebrow = 迁移结果
outcome-title = 欢迎使用你的新笔记本电脑。
outcome-body = 以下是你同意的每项内容的结果。
outcome-title-old = 你的东西已在新笔记本电脑上。
outcome-body-old = 以下是你发送的每项内容的结果。这台笔记本电脑保持原样。
outcome-files-title = 已选文件
outcome-status-partial = 部分完成
# Added 23 October 2026: the after-move check on R01, counted the way the scan counts.
outcome-kinds-title = 迁移后已检查
outcome-kind-photos = 照片
outcome-kind-videos = 视频
outcome-kind-music = 音乐
outcome-kind-documents = 文档
outcome-kind-count = { $planned } 个中已复制并检查 { $copied } 个
outcome-next-title = 接下来要做什么
outcome-old-safe = 未完成的内容会一直列在这里。旧笔记本电脑上没有删除任何东西。
outcome-landed-title = 你的东西现在在哪里
outcome-report = 查看完整报告
outcome-home = 返回首页
outcome-person-copied =
    { NUMBER($count) ->
       *[other] 已复制 { $count } 个文件。
    }
outcome-person-not-done =
    { NUMBER($count) ->
       *[other] { $count } 个未完成。
    }
outcome-person-at-sign-in = { $name } 下次登录这台笔记本电脑时，这些内容就会放好。
move-fallback-folder = 来自旧笔记本电脑
move-stop = 停止移动
move-before-open = 等待时：离开这台笔记本电脑之前
move-get-help = 获取帮助
pickup-eyebrow = 你的移动
pickup-title = 安全地继续。
pickup-body = 这台笔记本电脑上的一次移动中途停止了，可能是因为它重启或断电。请先整理，然后重新配对两台笔记本电脑以继续。
pickup-unfinished-title = 未完成的复制
pickup-unfinished-body =
    { NUMBER($count) ->
       *[other] 移动停止时，有 { $count } 个文件正在复制。
    }
pickup-unfinished-badge = 待整理
pickup-tidy = 整理
pickup-tidying = 正在整理。已完成的副本会经过检查并放到位；你的任何内容都不会被更改。
pickup-done = 整理完毕。重新配对两台笔记本电脑以继续：这里已有的内容会被识别，不会重复复制。
pickup-failed = 这次整理没有成功。你的任何内容都没有被更改。请重试，或先在这台笔记本电脑上关闭 PCTwin 再重新打开。
pickup-put-in-place = 已放到位
pickup-kept = 保留以继续
pickup-not-finished =
    { NUMBER($count) ->
       *[other] 有 { $count } 个文件无法完成。下次移动会重新复制。
    }
pickup-left-for-later =
    { NUMBER($count) ->
       *[other] 有 { $count } 个文件位于当前未连接的硬盘上。请连接该硬盘后再整理一次。
    }
pickup-later = 以后再说
ready-eyebrow = 欢迎
ready-title = 你的内容已准备好。
ready-body = 你旧笔记本电脑上的内容已移到这台电脑，现在就在你自己的文件夹里。
ready-landed = 个文件在你的文件夹中
ready-where-title = 它们在哪里
ready-where-body = 在“文档”“图片”“音乐”“视频”等你常用的文件夹中，和旧笔记本电脑上一样。
ready-kept =
    { NUMBER($count) ->
       *[other] 有 { $count } 个文件被单独保存，因为这里已有同名文件。两者都已保留。
    }
ready-failed =
    { NUMBER($count) ->
       *[other] 有 { $count } 个文件无法放入你的文件夹。请让完成移动的人再移动一次。
    }
ready-done = 完成
move-status-kept = 已保留
move-wait-eyebrow = 你的移动
move-wait-scanning-title = 正在查找你的文件。
move-wait-scanning-body = 旧笔记本电脑正在查看你的文件夹。它只读取：不会打开、更改或删除任何内容。
move-found = 目前已找到：你的内容 { $size }。
move-wait-approval-title = 正在等待新笔记本电脑。
move-wait-approval-body = 你的文件已准备好。在新笔记本电脑上查看计划并批准即可开始。
move-wait-list-title = 正在等待旧笔记本电脑。
move-wait-list-body = 你的旧笔记本电脑正在查找你的东西，然后在那台电脑上选择要搬的内容。完成后，要过来的内容会显示在这里，等你接受。这可能需要几分钟。
move-wait-starting-title = 正在开始移动。
move-wait-starting-body = 计划已批准。复制马上开始。
move-wait-admin-title = 正在等待管理员。
move-wait-admin-body = 这台笔记本电脑正在请求管理员批准，以便同时移动其他人的内容。请在电脑打开的窗口中作答。
move-others-not-approved = 管理员没有批准，所以这次只移动你的内容。其他人的内容保持原样。
move-others-failed = PCTwin 无法启动移动其他人内容所需的组件，所以这次只移动你的内容。他们的任何内容都没有被更改。
move-others-not-here = 此版本暂不支持从这台笔记本电脑移动其他人的内容，所以只移动你的内容。
move-others-allow-in-settings = 首先，管理员需要在刚刚打开的“系统设置”页面中允许 PCTwin。这次只移动你的内容；允许 PCTwin 之后，下一次移动也可以带上其他人的内容。
approve-prompt = PCTwin 将读取 { $names } 的文件，把它们移到新的笔记本电脑。这台笔记本电脑的管理员只批准这一次移动。
who-here-title = 谁要搬过去？
who-here-body = 你的内容总会被移动。勾选这台笔记本电脑上其他也需要移动内容的人。PCTwin 从不向你显示他们的文件。
who-here-you = { $name }（你）
who-here-you-hint = 始终移动
who-here-approval = 移动其他人的内容需要管理员批准，你勾选的所有人只需批准一次。电脑会在它自己的窗口中请求。
who-here-not-here = 此版本暂不支持从这台笔记本电脑移动其他人的内容。只移动你的内容。
place-title = 他们的内容放到哪里？
place-body = 旧笔记本电脑上也有其他人的内容。请为每个人选择这台电脑上的一个账户、为其新建账户，或不移动。你永远看不到他们的文件。
place-size =
    { NUMBER($count) ->
       *[other] { $count } 个文件，{ $size }
    }
place-select = { $name } 的内容放到哪里
place-note = 新账户以及在他们登录前保管其内容，需要在你批准计划后由管理员批准一次。每个新账户的临时密码只会在电脑自己的窗口中显示一次。
place-todo =
    { NUMBER($count) ->
       *[other] 还需为 { $count } 个人选择内容放到哪里。
    }
fda-title = 你的 Mac 正把一些文件夹设为私密。
fda-body =
    { NUMBER($count) ->
       *[other] 由于 Mac 的隐私设置，你有 { $count } 个位置无法查看。请允许 PCTwin 访问，然后再检查一次，以免遗漏你的内容。
    }
fda-step-1 = 打开“系统设置”，然后选择“隐私与安全性”。
fda-step-2 = 选择“完全磁盘访问权限”。
fda-step-3 = 打开 PCTwin。Mac 可能会要求输入密码。
fda-open = 打开这些设置
fda-note = PCTwin 只读取你的文件夹来复制它们，从不更改这台笔记本电脑上的任何内容。
fda-again = 再检查一次
fda-without = 不含这些继续
password-title = PCTwin：一个新账户
password-body =
    { $name } 现在在这台笔记本电脑上有了账户。
    
    登录名：{ $account }
    临时密码：{ $password }
    
    请把这些交给 { $name }。首次登录时，他们会设置自己的密码。请现在记下来：PCTwin 不会保存它。
move-ended-eyebrow = 你的移动
move-ended-kept-title = 已复制的内容
move-ended-kept-body = 在此之前已复制并检查的内容都会留在新笔记本电脑上。
move-ended-old-title = 你的旧笔记本电脑
move-ended-old-body = 上面的任何内容都没有被更改或删除。
move-ended-old-badge = 未更改
move-ended-next = 要重试，请重新配对两台笔记本电脑。已复制并检查的文件会被识别，不会重复复制。
move-lost-title = 与另一台笔记本电脑的连接已断开。
move-lost-body = 两台笔记本电脑无法再互相连接，所以移动无法继续。请确认两台都已开机、未休眠且彼此靠近。
move-not-approved-title = 移动没有开始。
move-not-approved-old = 新笔记本电脑上没有批准计划，所以没有复制任何内容。
move-not-approved-new = 你没有批准计划，所以没有复制任何内容。
move-failed-title = 出了点问题。
move-failed-body = 移动无法继续。如果再次发生，下方的“更多详情”会说明出了什么问题。
move-peer-system-windows = Windows
move-peer-system-mac = Mac
move-peer-system-linux = Linux
move-peer-system-other = 其他系统
move-update-here-title = 请在这台电脑上更新 PCTwin
move-update-here-body = 这台电脑上的 PCTwin 比另一台旧，所以它们无法一起搬文件。请在这里更新 PCTwin，然后重新开始。没有复制任何内容。
move-update-there-title = 请在另一台电脑上更新 PCTwin
move-update-there-body = 另一台电脑上的 PCTwin 比这台旧，所以它们无法一起搬文件。请在那台电脑上更新 PCTwin，然后重新开始。没有复制任何内容。
move-refused-title = 移动无法开始。
move-refused-not-paired = 两台笔记本电脑已不再配对。请重新配对以开始移动。
move-refused-busy = 这台笔记本电脑上已有一个移动正在进行。
move-refused-not-moving = 这台笔记本电脑上没有正在进行的移动。
move-refused-plan-changed = 计划在批准前已更改。请重新配对两台笔记本电脑以查看新计划。
move-refused-failed = 无法为移动准备好这台笔记本电脑的文件夹。请重新配对两台笔记本电脑再试一次。
drive-problem-title = 移动需要你处理。
drive-problem-body = PCTwin 多次检查后发现，保存在 { $place } 中的副本与原文件不一致。该硬盘可能出现故障、已满或已断开，因此复制到该处已暂停。
drive-problem-kept-title = 已复制并检查
drive-problem-waiting-title = 仍需复制到 { $place }
drive-problem-badge = 已暂停
drive-problem-fix = 请确认硬盘已连接且有可用空间，然后继续。不会为了绕过问题而遗漏你选择的任何内容。你也可以停止移动。
drive-problem-carry-on = 继续

## R02 Your move, item by item (detailed report)

report-title = 你的搬迁，按类型。
report-body = 打开一个类型即可查看其中的文件。需要你处理的内容排在最前。
report-needs-title = { $count ->
       *[other] { $count } 项需要你处理
    }
report-group-photos = 照片
report-group-videos = 视频
report-group-music = 音乐
report-group-documents = 文档
report-group-other = 其他文件
report-group-size = { $files ->
       *[other] { $files } 个文件
    } · { $size }
report-group-done = 全部已复制并检查
report-group-some = { $total } 个中的 { $done } 个已复制并检查
report-group-left = { $count ->
       *[other] 按你的选择，{ $count } 个留在旧笔记本电脑上
    }
report-filter-label = 显示
report-col-item = 项目
report-col-result = 结果
report-col-details = 检查或下一步
report-from = 来源：{ $path }
report-to = 去向：{ $path }
report-none = 没有项目是这个结果。
report-more-details = 更多详情
# Added 23 October 2026: what the report says beside a line's result (changed since the scan).
report-note-changed-since-scan = 扫描后有改动：已按现在的样子复制。
report-note-no-longer-there = 已不在原处：发送之前，它已在旧笔记本电脑上被移动、改名或删除。
report-note-time-not-kept = 日期未保留：这个磁盘无法保存它的最后修改日期，所以显示的是复制的日期。

evidence-read-back = 已从硬盘读回进行检查。
evidence-memory = 通过这台笔记本电脑的内存检查，而不是硬盘。
evidence-already-there = 新笔记本电脑上已有：同一个文件已经在那里，所以没有再次复制。
evidence-recovered = 在笔记本电脑重启前检查过，重启后又检查了一次。
evidence-new-laptop = 已由新笔记本电脑检查。
evidence-memory-explain = 副本已读回并且一致，但读取经过了这台笔记本电脑的内存，所以读到的内容可能来自内存而不是硬盘。硬盘本身的故障可能不会在这里显示，所以打开一次文件是个不错的检查方法。
evidence-not-checked = 未检查，因为没有完成。
# Machine-translated 16 October 2026.
why-memory-cannot-skip-cache = 这个硬盘不允许PCTwin直接读取它，所以检查是经过这台笔记本电脑的内存完成的。有些网络硬盘和USB闪存盘就是这样。
why-memory-not-shown = 这个硬盘无法证明PCTwin的读取真的到达了它，所以这次检查算作经过这台笔记本电脑的内存完成。
why-memory-no-way = 这台笔记本电脑的系统没有办法直接读取这个硬盘，所以检查是经过它的内存完成的。
why-memory-stored-specially = 这个文件以特殊方式存储，比如经过压缩或加密，所以系统通过内存读取它。
why-memory-too-small = 这个文件很小，所以硬盘把它和自己的记录放在一起，系统通过内存读取它。

reason-not-enough-space = 新笔记本电脑空间不足。请在上面腾出空间，然后开始一次包含这个项目的新迁移。
reason-changed-during-move = 复制期间，这个文件在旧笔记本电脑上一直在变化。请关闭正在使用它的应用，然后开始一次包含它的新迁移。
reason-unreadable = 旧笔记本电脑不允许PCTwin读取这个项目。请检查它在旧电脑上能否打开，然后开始一次包含它的新迁移。
reason-not-verified = 副本与原文件不一致，所以没有保留。请开始一次包含它的新迁移。如果再次失败，原文件可能已损坏。
reason-refused-by-safety-check = 由于名称、位置或大小的原因，新笔记本电脑的安全检查拒绝了这个项目。原文件仍在旧电脑上，如有需要，你可以用其他方式复制它。
reason-name-not-allowed = 这个名称在新笔记本电脑上不能使用，而且无法生成安全的新名称。请在旧电脑上重命名，然后开始一次包含它的新迁移。
reason-install-failed = 这个应用没有安装成功。请从开发者的官网安装。
reason-no-equivalent = 这个应用没有适用于新笔记本电脑系统的版本。请找一个能在那里使用的类似应用。
reason-drive-locked = 目标硬盘已锁定。请先解锁（例如用它的 BitLocker 密码或恢复密钥），然后再移动一次。
reason-sign-in = 已放到位，但你需要登录。请打开它并登录。
reason-activation = 应用已放到位，但需要在新笔记本电脑上使用许可证。请打开它，用你的许可证或账户激活。
reason-permission = 新笔记本电脑需要先授予权限。请打开这个项目，在询问时选择“允许”。
reason-online-only = 这个文件仅在线存储，所以旧笔记本电脑上没有可复制的文件。请在新电脑上，通过在线存储它的应用下载。
reason-manual-install = 这个应用必须从开发者的官网获取。请到那里下载并安装。
reason-restart = 新笔记本电脑需要重启才能完成。准备好后请重启。
reason-finish-by-hand = PCTwin无法替你完成这一步。请按照这份报告中为它列出的步骤操作。
reason-parts-missing = 这个文件夹里有些文件没有到达，所以它在新电脑上并不完整。这份报告列出了这些文件及原因。请再次发送它们，或用其他方式复制这个文件夹。
reason-moved-file-missing = 这个设置指向的文件或文件夹不在新笔记本电脑上（不在迁移给它的名称下），所以 PCTwin 没有改动这个设置。请打开该应用，重新选择这个文件。

## Item results (every item ends with one of these)

status-copied = 已复制
status-reinstalled = 已重新安装
status-converted = 已转换
status-needs-action = 需要处理
status-skipped-by-user = 已由你跳过
status-failed = 失败

## Screens: words the copy deck did not cover (same voice)

progress-minutes = { $low }到{ $high }分钟
# Machine-translated 16 October 2026.
progress-minutes-one =
    { NUMBER($minutes) ->
       *[other] { $minutes }分钟
    }
progress-speed-range = { $low }到{ $high } { $unit }
progress-count-failed =
    { NUMBER($count) ->
       *[other] { $count }个项目无法复制
    }
progress-queue-empty = 还没有开始任何内容。
progress-approved-job = 已同意的迁移
progress-route-old = 旧笔记本电脑
progress-route-new = 新笔记本电脑
progress-panel-pausing = 正在暂停
outcome-metric-size = 已复制并检查
outcome-metric-action = 需要你处理
progress-percent-unknown = PCTwin仍在计算这次迁移的大小。
mini-title-send-paused = 发送已暂停。
cancel-ack-hint = 勾选上面的方框，即可启用停止按钮。
progress-state-placing = 正在放到位
progress-state-existing = 已存在
outcome-stopped-title = 你的迁移已停止。
outcome-stopped-body = 你停止了迁移。在此之前已复制并检查的内容会保留，其余的没有复制。
outcome-broke-title = 你的迁移提前结束了。
outcome-broke-body = 两台笔记本电脑之间出了问题，迁移无法继续。在此之前已复制并检查的内容会保留，其余的没有复制。
outcome-files-summary = { $total }个项目中，已复制并检查{ $done }个。
outcome-files-memory =
    { NUMBER($count) ->
       *[other] { $count }个是通过这台笔记本电脑的内存检查的，而不是硬盘。
    }
report-show-more = 显示更多
report-showing = 显示{ $total }项中的{ $shown }项

## Component kit

dialog-close = 关闭
toast-dismiss = 知道了
code-label = 六位配对码

## Shared Quick Send words

qs-eyebrow = 快速发送
qs-eyebrow-receiving = 在你的笔记本电脑上接收
qs-label-this = 这台笔记本电脑
qs-label-other = 另一台笔记本电脑
qs-pick-failed = 无法打开选择窗口。请重试。
qs-pick-too-many = 文件太多，一次发送装不下。请取消勾选一些文件夹，或分几次发送。
qs-problem-lost-title = 两台电脑之间的连接已断开。
qs-problem-failed-title = 这次未能发送文件。
qs-problem-body = 已接收并检查的内容会保留在原处。重新配对两台笔记本电脑即可再试一次。
qs-route =
    .aria-label = 从{ $from }到{ $to }
qs-route-sender = 正在发送
qs-route-receiver = 接收 · 同一个Wi-Fi
qs-tally =
    { NUMBER($count) ->
       *[other] 已选{ $count }个项目 · { $size }
    }
qs-tally-badge = 附近发送

## Q01 Quick Send home

qs-home-title = 把文件发送到附近的笔记本电脑。
qs-home-body = 或者从附近的笔记本电脑接收文件。
qs-send-title = 发送文件
qs-send-body = 选择你笔记本电脑上的文件或文件夹。
qs-receive-title = 接收文件
qs-receive-body = 让附近的笔记本电脑把文件发送到你的电脑。
qs-home-action = 选择文件
qs-receiving = 已准备好接收。附近的笔记本电脑看到你的电脑名为{ $label }。
qs-receive-stop = 停止接收
qs-no-nearby = 附近没有找到笔记本电脑。请在发送方电脑上打开PCTwin，并让两台电脑连接同一个Wi-Fi。

## Q02 Choose files and folders

qs-choose-title = 你要发送什么？
qs-choose-body = 只选择你想分享的文件和文件夹。
qs-drop =
    .aria-label = 文件和文件夹拖放区
qs-drop-title = 把文件或文件夹拖到这里
qs-drop-body = 或者使用“添加文件”或“添加文件夹”来添加。
qs-drop-active = 松开即可添加。
qs-add-files = 添加文件
qs-add-folders = 添加文件夹
qs-remove =
    .aria-label = 移除{ $name }
qs-file-detail = { $size } · { $kind }
qs-empty = 还没有选择任何内容。请添加要发送的文件或文件夹。
qs-online-only = 有些文件只在线上。请先让它们在你的笔记本电脑上可用，然后再发送。
qs-protected = PCTwin无法读取{ $name }。请允许访问，或将它从列表中移除。
qs-allow = 允许访问
qs-choose-continue = 继续

## Q12 Send progress

qs-sending-title = 你的文件正在发送中。
qs-sending-body = 副本直接发送到接收方电脑。
qs-sending-progress =
    .aria-label = 正在发送文件
qs-sending-current = 正在发送：{ $name }
qs-sending-speed = { $speed }（实测）
qs-step-copying = 正在复制
qs-step-copying-body =
    { NUMBER($done) ->
       *[other] 已发送{ $done }个文件。
    }
qs-step-checking = 正在检查
qs-step-checking-body = 每个文件都会与原文件进行比对。
qs-sending-paused = 已暂停。已接收并检查的文件会保留。
qs-sending-disconnected = 与{ $label }的连接已断开。已检查的文件会保留。正在自动重新连接……
qs-sending-needs-action =
    { NUMBER($count) ->
       *[other] { $count }个项目需要你处理。
    }
qs-pause = 暂停发送
qs-resume = 恢复
qs-compact = 精简显示
qs-stop = 停止这次发送
qs-sending-complete-title = 发送已完成

## Q20 Stop this send? (and decline)

qs-stop-title = 停止这次发送？
qs-stop-body = 停止之前，请检查哪些已经送达。
qs-stop-arrived-title = 完整送达的文件
qs-stop-arrived-body = 留在你选择的文件夹中。
qs-stop-rest-title = 尚未发送的文件
qs-stop-rest-body = 停止后不会再发送任何新内容。
qs-stop-partial-title = 部分送达的文件
qs-stop-partial-body = 绝不会以真实名称保存，所以不会被误认为完整的副本。
qs-stop-status-keep = 保留
qs-stop-status-stop = 停止
qs-stop-status-review = 待查看
qs-stop-ack = 停止这次发送
qs-stop-ack-hint = 完整送达的文件不会被删除。
qs-stop-keep = 继续发送

## F08 Your moves, kept on this laptop (history)

history-title = 你的迁移记录，保存在这台笔记本电脑上。
history-body = 再次打开报告，或继续已保存的迁移。
history-row-move = { $from } → { $to }
history-status-finished = 已完成
history-status-steps-left =
    { NUMBER($count) ->
       *[other] 已完成，还有{ $count }个步骤需要你处理
    }
history-status-paused = 已暂停
history-status-stopped = 已安全停止
history-open-report = 报告
history-undo = 撤销
history-continue = 继续
history-export = 导出历史记录
history-exported = 历史记录已保存为{ $path }。
history-export-failed = 无法将历史记录保存到那里。请选择其他文件夹，然后再次尝试。
history-note = 这份历史记录只保存在这台笔记本电脑上。除非你自己导出并分享，否则不会发送到任何地方。
history-empty = 你的第一次迁移及其报告会显示在这里。
history-empty-title = 还没有迁移记录
history-start = 开始新的迁移

## Quick Send: words the mockups do not have (added while building; same voice)

qs-soon-note = 这个版本还没有开启快速发送。没有发送或接收任何内容。
qs-tally-counting =
    { NUMBER($count) ->
       *[other] 已选{ $count }个项目 · 仍在计算大小
    }
qs-gone = { $name }已不在原处。它可能已被移动或删除。请取消勾选，或者把它放回原处后重新添加。
qs-choose-why-problems = 部分已勾选的项目暂时无法发送。请取消勾选，或按说明解决后继续。
qs-sending-card = 正在发送文件
qs-progress-of = 共 { $total }
history-open-name = { $what }：{ $title }

## R05 撤销：撤销在每个硬盘上会做什么，以及每个副本的结果

undo-here-removes = 撤销会删除 PCTwin 制作、你没有改动过的副本。你改动过的内容都会保留。
undo-here-to-trash = 撤销会把 PCTwin 制作、你没有改动过的副本移到废纸篓。你改动过的内容都会保留，也不会删除任何东西。
undo-here-keeps-fat = 撤销会保留 { $drive } 上的每个副本。这个硬盘使用 FAT 或 exFAT，它会重复使用区分文件的编号，所以 PCTwin 无法确定哪些文件是它自己的。你可以自己删除它们。
undo-here-keeps-other = 撤销会保留 { $drive } 上的每个副本。在这种硬盘上，PCTwin 无法安全地移动或删除文件。你可以自己删除它们。
undo-here-more = 更多详情
undo-here-more-trash = Mac 无法让 PCTwin 确认没有其他应用还在写入某个文件，所以 PCTwin 从不删除文件。它会把每个副本移到同一个硬盘上的废纸篓。你可以从废纸篓把它放回原处。
undo-here-more-fat = 在 FAT 和 exFAT 硬盘上，新文件可能会得到某个已删除文件用过的编号。PCTwin 只处理它能证明属于自己且未改动的文件，所以会保留这些文件。
undo-done-trashed = 已移到废纸篓，名称为“{ $name }”。你可以从废纸篓把它放回原处。
undo-kept-fat = 已保留。这个硬盘（FAT 或 exFAT）会重复使用区分文件的编号，所以 PCTwin 无法确定这是它的副本。如果你不需要它，可以自己删除。
undo-kept-no-trash = 已保留。PCTwin 无法把这个副本移到它所在硬盘上的废纸篓。如果你不需要它，可以自己删除。

## Quick Send (26 October 2026): pairing in Quick Send's words, where received files go, and the Done page

qs-pair-old-eyebrow = 正在发送
qs-pair-old-body = 在接收方电脑上选择“接收文件”，然后输入这个配对码。
qs-pair-old-waiting = 正在等待接收方电脑……
qs-pair-old-label-hint = 接收方电脑会通过这个名字找到你的电脑。
qs-pair-old-steps-title = 在接收方电脑上
qs-pair-old-step-1 = 打开PCTwin，选择“快速发送”，然后选择“接收文件”。
qs-pair-old-left = 在选择数字之前，接收方电脑停止了配对，因此没有共享任何内容。这是新的配对码。
qs-pair-network-hint = 选择接收方电脑所连接的网络。
qs-pair-choose-body = 看看接收方电脑。选择它显示的数字。
qs-pair-choose-renewed = 已显示新的数字。请选择接收方电脑上的那个。
qs-pair-choose-note = 如果接收方电脑没有显示数字，或者这些数字都不匹配，请选择“都不匹配”。
qs-pair-heads-up-firewall = Windows可能会询问PCTwin能否接受连接。请选择“允许”，这样接收方电脑才能连接到你的电脑。
qs-pair-new-eyebrow = 正在接收
qs-pair-new-searching-title = 正在搜索发送方电脑……
qs-pair-new-searching-body = 请在发送方电脑上保持PCTwin打开，并与你的电脑连接同一个Wi-Fi。
qs-pair-new-search-failed-title = 无法查找发送方电脑。
qs-pair-new-list-title = 哪一台笔记本电脑在发送？
qs-pair-new-list-body = 选择发送方电脑屏幕上显示的名字。
qs-pair-new-same-label = 附近另一台笔记本电脑也叫这个名字。请查看发送方电脑的屏幕。
qs-pair-new-none-title = 没有找到正在发送的笔记本电脑。
qs-pair-new-none-body = 请检查发送方电脑上的PCTwin是否已打开并显示配对码，以及两台电脑是否连接同一个Wi-Fi。然后重新搜索。
qs-pair-new-code-hint = 请从{ $label }的屏幕上读取。它每分钟都会变化。
qs-pair-new-number-title = 在{ $label }上选择这个数字。
qs-pair-new-number-body = { $label }上会显示三个数字。请在那里选择这个。
qs-pair-new-number-waiting = 正在等待{ $label }……
qs-pairing-code-invalid = 配对码需要六位数字。请查看{ $label }的屏幕，然后重新输入。
qs-pairing-code-wrong = 配对码不正确。请查看{ $label }的屏幕，然后再次尝试。
qs-pairing-code-expired = 配对码已更换。请输入{ $label }屏幕上的新配对码。
qs-pairing-busy = 现在无法与{ $label }配对：它正在与另一台电脑配对。请稍等片刻，然后再次尝试。
qs-pairing-wait = { $label }正在准备新的配对码。请等待它准备好，然后输入新配对码。
qs-pairing-locked = 错误尝试次数过多，配对已停止。请在{ $label }上选择“重新开始”。
qs-pairing-unreachable = 你的笔记本电脑无法连接到{ $label }。请检查两台电脑是否连接同一个Wi-Fi，然后再次尝试。
qs-pairing-wrong-number = 这个数字与接收方电脑上的不匹配。没有共享任何内容。请在接收方电脑上输入新的配对码，再次尝试。
qs-pairing-no-match = { $label }上的数字不匹配。没有共享任何内容。请输入它屏幕上的新配对码。
qs-pairing-timed-out = 没有及时选择数字。请使用{ $label }上的新配对码再次尝试。
qs-pairing-connection-lost-new = 与{ $label }的连接已断开。没有共享任何内容。
qs-pairing-choose-network = 你的笔记本电脑连接了多个网络。请选择接收方电脑所连接的那个：
qs-pairing-failed-new = 这次配对没有成功。没有共享任何内容。请输入{ $label }屏幕上的新配对码。
qs-folder-line = 文件将保存在{ $folder }。
qs-folder-change = 更改
qs-folder-title = 文件要保存到哪里？
qs-folder-none = PCTwin无法使用接收文件的文件夹。请为文件选择一个文件夹，然后再连接。
qs-folder-choose = 选择文件夹
qs-folder-failed = PCTwin无法在那个文件夹中保存文件。请选择另一个。
qs-done-sent-title = 你的文件已发送。
qs-done-here-title = 你的文件已到达。
qs-done-partial-title = 有些文件没有到达。
qs-done-stopped-title = 这次发送已停止。
qs-done-empty-title = 没有发送任何内容。
qs-done-arrived =
    { NUMBER($count) ->
       *[other] { $count }个文件 · { $size } 已到达，并与原文件一致
    }
qs-done-arrived-count =
    { NUMBER($count) ->
       *[other] { $count }个文件已到达，并与原文件一致
    }
qs-done-already =
    { NUMBER($count) ->
       *[other] { $count }个文件已经存在且完全相同，因此没有再次发送。
    }
qs-done-stopped-body = 停止前到达的文件已经过检查，仍保存在原来的位置。没有发送其他内容。
qs-done-folder = 已保存在{ $folder }
qs-done-open-folder = 打开文件夹
qs-done-dates-new =
    { NUMBER($count) ->
       *[other] 这{ $count }个文件显示的是到达时间，所以排在文件夹最上面。
    }
qs-done-keep-dates = 保留原始日期
qs-done-dates-kept =
    { NUMBER($count) ->
       *[other] { $count }个文件已恢复原始日期。
    }
qs-done-dates-failed = 无法恢复原始日期。文件完好无损，仍在原处。
qs-done-open-failed = 无法打开文件夹。它可能已被移动或删除。
qs-done-not-opened = PCTwin没有打开或运行任何接收的文件。
qs-done-problems-title = 需要注意的文件
qs-done-not-arrived = 它没有到达。
qs-done-not-completed = 停止前没有发送它。
qs-done-needs-action = 它已在这里，但尚未完成。具体做法见“更多详情”。
qs-done-mark-not-kept = 它已在这里，但这个磁盘无法把它标记为从另一台笔记本电脑下载的文件。请谨慎打开。
qs-done-see-files = 查看文件
qs-done-renamed = 已另存为{ $name }，因为那里已有同名文件。
qs-done-identical = 那里已有一个完全相同的文件，因此没有再次发送。
qs-done-send-missing = 发送缺少的文件
qs-done-send-missing-body = 只发送没有到达的文件。你需要先重新配对两台电脑。
qs-done-done = 完成
qs-stop-now = 停止发送
qs-stop-finished = 这次发送已经完成。
qs-route-receiving = 正在接收
qs-receiving-title = 文件正在到达。
qs-receiving-body = 副本直接来自{ $label }。
qs-receiving-card = 正在接收文件
qs-receiving-progress =
    .aria-label = 正在接收文件
qs-receiving-current = 正在接收：{ $name }
qs-step-receiving-body =
    { NUMBER($done) ->
       *[other] 已到达{ $done }个文件。
    }
qs-tally-files =
    { NUMBER($count) ->
       *[other] { $count }个文件 · { $size }
    }
qs-receive-pause = 暂停
qs-label-yours = 你的笔记本电脑
qs-label-receiving = 接收方电脑
qs-label-sending = 发送方电脑
qs-peer-receiving = 接收方电脑
qs-peer-sending = 发送方电脑
qs-pair-old-label-title = 你的笔记本电脑的名字
qs-pair-old-rename-label = 你的笔记本电脑的名字
qs-pair-old-step-2 = 如果出现列表，请选择你的笔记本电脑的名字。
qs-pair-old-locked-body = 输错五次配对码后，配对已停止，以保护你的笔记本电脑。没有共享任何内容。如果是你自己，请选择“重新开始”获取新的配对码。
qs-pair-new-needs-update = 它的PCTwin版本较旧。请在它上面更新PCTwin，然后重新搜索。
qs-pairing-no-network = 你的笔记本电脑没有连接Wi-Fi或有线网络。请连接后再次尝试。
qs-problem-room-title = { $folder }中的空间不足。
qs-problem-room-body = 这些文件需要{ $needed }，那里可用的空间为{ $room }。没有保存任何内容。请释放空间或选择另一个文件夹，然后再试一次。
qs-done-refused-title = 接收方电脑没有接收这些文件。
qs-done-refused-body = 它的空间可能不足。没有发送任何内容。

## 第二阶段屏幕（任务清单 2.8）：通用部分

guard-state-not-started = 还没开始
guard-state-preparing = 正在准备
guard-state-awaiting-person = 等你选择
guard-state-awaiting-system = 等你的系统批准
guard-state-awaiting-maker = 等应用的开发者或在线服务
guard-state-working = 正在处理
guard-state-checking = 正在检查已完成的内容
guard-state-applying = 正在放到位
guard-state-pause-requested = 会在下一个安全的时间点暂停。目前还没有暂停。
guard-state-paused = 已在安全的时间点暂停
guard-state-source-unavailable = 旧笔记本电脑或硬盘没有连接
guard-state-blocked = 需要先解决一个问题
guard-state-partial = 部分完成
guard-state-retrying = 正在重试
guard-state-reconciling = 正在检查进行到哪里
guard-state-failed = 没有成功
guard-state-complete = 已完成
guard-state-cancelled = 已停止。已经完成的部分会保留。
guard-refused-unavailable = PCTwin 暂时无法检查这一项。什么都没有做。请稍后再试。
guard-refused-wrong-job = 这是另一次迁移的内容。什么都没有做。
guard-refused-old-session = 这个屏幕已经过时，所以什么都没有做。下面是现在的实际情况。
guard-refused-plan-changed = 你查看时方案发生了变化，所以什么都没有做。请重新查看。
guard-refused-not-now = 这一步不能这样做。什么都没有做。
guard-refused-not-shown = 这个选项已不在这个屏幕上。什么都没有做。
guard-progress-label = 进度
guard-progress-mark = 已完成 { $percent }%
guard-progress-of = { $done }/{ $total }
guard-progress-unknown-total = 目前 { $done }。总数还不知道。
guard-progress-none = 还没有测量到任何内容。
guard-and-more =
    { $count ->
       *[other] 还有 { $count } 项，已列在报告里。
    }
guard-not-known-yet = 还不知道
guard-free = 可用 { $free }
guard-free-unknown = 可用空间未知
guard-items-size = { $items } 个项目，{ $size }
guard-try-again = 重试
guard-pause = 暂停
guard-resume = 继续
guard-check-again = 再次检查
guard-strip-plan = 你的方案
guard-strip-report = 你的报告
guard-strip-permission = 批准
guard-strip-rescue = 抢救
guard-strip-undo = 撤销
guard-strip-checking = 检查中

## X09 重启后的检查，以及状态不可用

guard-checking-title = 正在检查进行到哪里
guard-checking-body = PCTwin 正在查看关闭前已完成的内容。它弄清楚后，你的选项就会回来。
guard-unavailable-title = 状态不可用
guard-unavailable-body = PCTwin 暂时无法说明这次迁移在做什么。在弄清楚之前，不会把任何内容显示为已完成。请稍后再检查。

## 2.9 做任何更改之前

guard-changes-title = 做任何更改之前
guard-changes-body = 这是这次迁移会在这台笔记本电脑上更改的全部内容，以及撤销对每一项能做什么。目前还没有任何更改。
guard-changes-download = 从互联网下载
guard-changes-download-none = 无
guard-changes-permissions = 系统批准
guard-changes-one-prompt = 你的系统只会询问一次
guard-changes-no-prompt = 不需要
guard-changes-restore-point = Windows 还原点
guard-changes-restore-point-first = 先请求
guard-changes-restore-point-none = 不使用
guard-changes-install =
    { $count ->
       *[other] 要安装：{ $count } 个应用
    }
guard-changes-merge =
    { $count ->
       *[other] 加在现有内容旁边：{ $count } 个项目
    }
guard-changes-replace =
    { $count ->
       *[other] 要替换，并保留旧版本：{ $count } 个项目
    }
guard-changes-leave-alone =
    { $count ->
       *[other] 保持原样：{ $count } 个项目
    }
guard-changes-space = 最忙的时候每个硬盘上需要的空间
guard-changes-why-prompt = 你的系统为什么会询问
guard-changes-approve = 批准这些更改
guard-undo-legend-can = PCTwin 可以撤销。
guard-undo-legend-keeps = PCTwin 会保留。撤销不会动它。
guard-undo-legend-you = 需要你处理。只有它的服务或开发者才能撤销。
guard-undo-can-reverse = PCTwin 可以撤销
guard-undo-can-remove-app = PCTwin 可以删除这个应用。它不会动你创建的文件。
guard-undo-must-keep = 保留：它本来就在这里
guard-undo-needs-you-service = 需要你处理：只有在线服务才能撤销
guard-undo-needs-you-maker = 需要你处理：登录和许可证归开发者管
guard-drive-needs = 需要 { $needed }。可用 { $free }。
guard-drive-needs-free-unknown = 需要 { $needed }。这个硬盘没有说明还剩多少空间。
guard-drive-fits = 放得下
guard-drive-short = 还差 { $short }
guard-drive-unknown = 未知
guard-permission-read-other-people = 读取旧笔记本电脑上其他人的文件
guard-permission-make-accounts = 在这台笔记本电脑上创建他们的账户
guard-permission-shared-folders = 把内容放进你的账户无法更改的共享文件夹
guard-permission-install-for-everyone = 为这台笔记本电脑上的所有人安装应用
guard-permission-restore-point = 先创建一个 Windows 还原点

## X06 和 H10 批准之后有内容发生变化

guard-changed-title =
    { $count ->
       *[other] 你批准之后，有 { $count } 项发生了变化
    }
guard-changed-body = 只有这些需要重新看一下。其他内容仍然保持批准，会继续进行。
guard-changed-changed-on-old-laptop = 它在旧笔记本电脑上发生了变化。
guard-changed-app-updated = 这个应用自己更新了。
guard-changed-destination-changed = 它要去的位置发生了变化。
guard-changed-new-scope = 这是新增的：有另一个人或另一台笔记本电脑加入了这次迁移。
guard-changed-permission-ended = 它的批准已经失效。
guard-changed-leave-out = 不包含这些
guard-changed-approve = 重新批准这些

## M31、M05 新笔记本电脑上的空间

guard-space-title = 新笔记本电脑上的空间
guard-space-title-short = 新笔记本电脑的空间不够
guard-space-body = PCTwin 算过了每个硬盘上会放的全部内容，包括撤销所需的空间。你选的内容不会为了放得下而被丢掉。
guard-space-proposal = PCTwin 对放不下的内容的建议
guard-keep-off-leave-in-cloud = 留在它们的云端，在那里是安全的
guard-keep-off-offload-drive = 放到你的另一个硬盘上
guard-keep-off-leave-on-old-laptop = 留在旧笔记本电脑上。在它们迁移完之前请保留好它。
guard-offload-title = 你的另一个硬盘
guard-offload-connected = 已连接
guard-offload-not-connected = 未连接
guard-offload-approved = 你选了这个硬盘。上面的文件不在新笔记本电脑上：要打开它们，请连接这个硬盘。
guard-offload-ask = PCTwin 可以把一些内容放到这个硬盘上。它需要你单独同意使用这个硬盘。
guard-offload-use = 使用这个硬盘
guard-space-approve = 批准这个方案
guard-safe-in = 安全保存在 { $provider }
guard-safe-in-cloud = 安全保存在你的云端
guard-safe-in-detail =
    { $count ->
       *[other] 有 { $count } 个文件只存放在线上。你在新笔记本电脑上登录后，它们就会回来。
    }
guard-your-cloud = 你的云端

## M09 保留只存放在线上的文件的副本

guard-local-title = 要保留你只存放在线上的文件的副本吗？
guard-local-body =
    { $count ->
       *[other] { $provider } 里有 { $count } 个文件只存放在线上。你也可以在新笔记本电脑上保留一份副本。
    }
guard-local-download = 要在旧笔记本电脑上下载
guard-local-old-space = 旧笔记本电脑上的空间
guard-local-how = PCTwin 每次取下来几个，发送出去，再把它们在旧笔记本电脑上改回只存放在线上。
guard-local-paused-metered = 已暂停：这个连接可能按流量收费。换成免费连接后会自动继续，你允许的话也可以现在继续。
guard-local-paused-no-space = 已暂停：旧笔记本电脑没有空间放接下来的几个文件了。其余的仍然安全保存在 { $provider }。
guard-local-done = 已完成。副本已在新笔记本电脑上。
guard-local-keep = 保留副本
guard-local-leave = 留在 { $provider }
guard-local-allow-metered = 允许在这个连接上进行

## M07 谁去哪里

guard-owners-title = 谁去哪里
guard-owners-body = 每个人的内容都会放进他们自己在这台笔记本电脑上的账户。仅凭名称相同，永远不会决定这一点。
guard-owners-from = 来自 { $laptop }
guard-owners-to-account = 放进这台笔记本电脑上的账户 { $account }
guard-owners-new-account = 放进这台笔记本电脑上的新账户
guard-owners-own-folder = 他们自己的“来自 { $laptop }”文件夹，因为名称有冲突
guard-owners-usual-folders = 他们平时用的文件夹
guard-owners-approve = 批准

## X12 没有完成的内容

guard-report-title-partial = 大部分内容已迁移。有几项没有。
guard-report-title-done = 这一部分的全部内容都已迁移。
guard-report-body-partial = 下面的内容仍在旧笔记本电脑上。准备好后，请逐项再试一次。已经迁移的内容会保留。
guard-report-body-done = 没有需要重试的内容了。
guard-line-app = 应用 { $name }
guard-line-items =
    { $count ->
       *[other] { $count } 个项目
    }
guard-line-settings =
    { $count ->
       *[other] { $count } 项设置
    }
guard-line-why-failed = 来自 { $laptop }：没有完成。
guard-line-why-laptop-not-connected = 来自 { $laptop }：那台笔记本电脑没有连接。请把它连到同一个 Wi-Fi，并保持开机。
guard-line-why-waits-for-another = 来自 { $laptop }：它在等另一个没有完成的步骤。请先解决那一个。
guard-line-why-not-started = 来自 { $laptop }：迁移还没开始就停了。

## M25、H10 系统的批准，以及还原点

guard-permission-title-explain = PCTwin 需要你的系统批准一次
guard-permission-title-waiting-for-system = 请回答你的系统的提问
guard-permission-title-denied = 你选了不同意
guard-permission-title-closed-without-answer = 提问被关闭了，没有回答
guard-permission-title-given = 已批准用于这次迁移
guard-permission-title-ended-by-sleep = 笔记本电脑进入睡眠后，批准失效了
guard-permission-title-ended-other-session = 有其他人登录后，批准失效了
guard-permission-body-explain = 你的系统会为整个迁移只询问一次。只有下面列出的步骤会用到它，它会随迁移结束而失效。
guard-permission-body-waiting-for-system = 你的系统自己的窗口已打开。如果它要求，请在那里输入密码。
guard-permission-body-denied = 只有需要它的步骤会被略过。其他内容继续进行。
guard-permission-body-closed-without-answer = 关闭窗口不等于同意，所以需要它的内容都没有做。你可以再次请求，或者不用它继续。
guard-permission-body-given = PCTwin 只会把它用于下面列出的步骤，它会随这次迁移结束而失效。
guard-permission-body-ended-by-sleep = 此后需要它的内容都没有做。要继续，请再次请求。
guard-permission-body-ended-other-session = 此后需要它的内容都没有做。等你回到自己的账户后，请再次请求。
guard-permission-never-password = PCTwin 从不索要你的密码，也永远看不到它。只有你的系统能看到。
guard-permission-not-now = 不用它继续
guard-permission-ask = 前往你的系统
guard-permission-ask-again = 再次请求
guard-restore-title = Windows 还原点
guard-restore-made = 已在 { $time } 创建。
guard-restore-earlier-exists = Windows 已有一个 { $time } 的还原点，所以没有再创建。
guard-restore-not-chosen = 未创建：你选择了不创建。
guard-restore-not-made-not-windows = 未创建：只有 Windows 才有还原点。
guard-restore-not-made-protection-off = 未创建：这个硬盘的 System Protection 是关闭的。你可以在 System Properties 里把它打开。
guard-restore-not-made-needs-admin = 未创建：需要你的系统批准。
guard-restore-not-made-unconfirmed = 未创建：Windows 说可以，但找不到任何还原点。
guard-restore-not-made-failed = 未创建：Windows 没能创建。

## T01 到 T04 从无法启动的笔记本电脑中抢救

guard-rescue-title-intro = 从无法启动的笔记本电脑里抢救重要的内容
guard-rescue-title-pick-drive = 选择要抢救的硬盘
guard-rescue-title-locked = 这个硬盘已锁定
guard-rescue-title-no-system = 这不是笔记本电脑的硬盘
guard-rescue-title-salvage = 以下是能读取到的内容
guard-rescue-body-intro = 把硬盘从旧笔记本电脑里取出来，用硬盘转接器或硬盘盒连到这台笔记本电脑上。
guard-rescue-body-pick-drive = 选择旧笔记本电脑的硬盘。存放内容的硬盘不能是你要抢救的这个。
guard-rescue-body-locked = 请先在你的系统里解锁它，然后再检查。
guard-rescue-body-no-system = 这个硬盘上没有笔记本电脑的系统和个人文件夹。如果它是普通的 USB 硬盘，请把它用在新笔记本电脑上。
guard-rescue-body-salvage = 看一下列表，然后使用能读取到的内容。
guard-rescue-can = 抢救能做什么
guard-rescue-can-body = 从仍能工作的硬盘中，读取你的文件，以及硬盘上找到的人员和设置。
guard-rescue-cannot = 它不能做什么
guard-rescue-cannot-body = 修复硬盘、读取已损坏的部分，或打开你没有密钥的硬盘。
guard-rescue-read-only-title = 这个硬盘只会被读取
guard-rescue-read-only-promise = PCTwin 只读取它。上面的任何内容都不会被更改、修复或格式化。
guard-rescue-system-read-only = 你的系统以只读方式打开了它，所以上面的内容不可能被更改。
guard-rescue-pctwin-reads-only = PCTwin 只读取了它。但你的系统是以可更改的方式打开它的，所以其他应用仍有可能更改它。
guard-rescue-refused-target = 那个硬盘是存放内容的地方，所以不能同时被抢救。请选择另一个硬盘。
guard-rescue-drive-detail = { $size }，{ $fs }
guard-rescue-drive-is-target = 这次迁移的内容会放到这个硬盘上。
guard-rescue-drive-cannot = 无法抢救
guard-rescue-choose = 抢救这个硬盘
guard-rescue-bit-locker-step-1 = 在 File Explorer 里打开这个硬盘。
guard-rescue-bit-locker-step-2 = Windows 要求时，输入它的恢复密钥。密钥通常保存在所有者的 Microsoft 账户里，网址是 aka.ms/myrecoverykey。
guard-rescue-bit-locker-step-3 = 回到这里，选择“再次检查”。
guard-rescue-file-vault-step-1 = 在 Mac 上用所有者的登录密码或恢复密钥解锁它。
guard-rescue-file-vault-step-2 = 回到这里，选择“再次检查”。
guard-rescue-apple-chip-step-1 = 这台 Mac 把硬盘锁定为只供它自己使用，所以把硬盘取出来没有用。
guard-rescue-apple-chip-step-2 = 如果这台 Mac 还能进入 Recovery，请用数据线配合 Share Disk。如果它完全无法启动，只有 Apple 的维修中心能帮忙。
guard-rescue-never-key = PCTwin 从不索要密钥，也从不保存它。
guard-rescue-readable =
    { $count ->
       *[other] 能读取 { $n } 个项目。
    }
guard-rescue-unreadable =
    { $count ->
       *[other] 有 { $count } 个项目无法读取
    }
guard-rescue-look = 查找硬盘
guard-rescue-check-again = 再次检查
guard-rescue-use = 使用能读取到的内容

## R05、R06 撤销

guard-undo-preview-title = 撤销会做什么
guard-undo-preview-body = PCTwin 刚刚查看了每一项更改。迁移之后你改动过的内容都会保留。在你选择“撤销”之前，什么都不会发生。
guard-undo-preview-which = 这会撤销在 { DATETIME($when, year: "numeric", month: "long", day: "numeric", hour: "numeric", minute: "2-digit") } 完成的那次迁移。你的其他迁移不会改变。
guard-undo-preview-which-earlier = 这会撤销用旧版 PCTwin 完成的那次迁移。你的其他迁移不会改变。
guard-undo-preview-admin = 要删除应用，这台电脑会请管理员批准。
guard-undo-plan-put-back = 恢复原样
guard-undo-plan-remove-app = PCTwin 会删除这个应用。PCTwin 不会动你创建的文件。
guard-undo-plan-keep-changed = 保留：你改动过它
guard-undo-plan-elsewhere = 需要你处理：在别处完成
guard-undo-plan-leave-alone = 不动：它本来就在这里
guard-undo-keep = 全部保持现状
guard-undo-run = 撤销
guard-undo-running-title = 正在撤销更改
guard-undo-done-title = 撤销做了什么
guard-undo-result-restored = 已还原
guard-undo-result-removed = 已删除
guard-undo-result-as-before = 本来就和之前一样
guard-undo-result-kept-changed = 已保留，因为你改动过
guard-undo-result-kept-other = 已保留，请看“更多详情”
guard-undo-result-kept-at = 已保留，名称为 { $name }
guard-undo-result-done-elsewhere = 在别处完成：只有它的服务或开发者才能撤销
guard-undo-result-not-done-yet = 还没完成。撤销下次会再试。

## The full move: apps, settings and app data (28 October 2026)

fullmove-plan-title = 应用、设置和应用数据
fullmove-plan-body = PCTwin 查看了你旧电脑上的应用、设置和应用数据。在你的文件之后，它会在这台电脑上做这些事。在你开始迁移之前，什么都不会改变。
fullmove-group-install = PCTwin 将安装的应用：{ $count }
fullmove-group-settings = PCTwin 将复制的应用设置：{ $count }
fullmove-group-needs = 需要你自己做的事：{ $count }
fullmove-group-here = 这台电脑上已有的应用：{ $count }
fullmove-settings-count = 设置：{ $count }。
fullmove-settings-merge = 会被添加，这台电脑上的内容不会被替换。
fullmove-settings-replace = 这台电脑上的一些设置会被替换。撤销可以把它们恢复。
fullmove-app-install = PCTwin 从开发者那里安装它并进行检查。
fullmove-app-here = 它已经在这台电脑上，所以 PCTwin 保持原样。
fullmove-step-copy = 已复制，可以撤销。
fullmove-step-export-first = 在旧电脑上使用应用自带的导出功能，然后在这台电脑上的应用里导入。
fullmove-step-sign-in = 在这台电脑上重新登录。
fullmove-step-relink = 在这台电脑上重新设置，或从应用里重新关联。
fullmove-step-not-yet = PCTwin 暂时还不迁移这个应用的数据。它仍安全地保存在你的旧电脑上。
fullmove-step-look-at-it = PCTwin 无法判断这是什么。请在旧电脑上查看。
fullmove-group-update =
    { $count ->
       *[other] 这台笔记本电脑上有较旧的版本
    }
fullmove-update-it = 更新 { $name }
fullmove-update-hint = PCTwin 会安装较新的版本，你的设置保持不变。
fullmove-special-not-yet = 应用数据，例如邮件、浏览器和游戏
fullmove-left-by-person = 应用、设置和应用数据：按你的选择留在旧笔记本电脑上
fullmove-special-finish = PCTwin 没有迁移这个。它仍安全地保存在你的旧电脑上。
fullmove-more = 还有 { $count } 项。
fullmove-not-moved-title = PCTwin 留在旧电脑上的设置
fullmove-not-moved-private = 隐私和安全设置以及登录信息，从不复制：{ $count }
fullmove-not-moved-could-run = 会启动或打开某些内容的设置，为安全起见保留：{ $count }
fullmove-not-moved-old-laptop-only = 只适用于旧电脑的设置：{ $count }
fullmove-not-moved-system-own = PCTwin 暂不迁移的系统自身设置：{ $count }
fullmove-not-moved-not-read = 已找到但未读取的设置：{ $count }
fullmove-special-not-moved = 留下的应用文件，例如缓存：{ $count }
fullmove-prompt = 这台电脑会请求一次管理员权限。
fullmove-prompt-restore = 这台电脑会请求一次管理员权限，并在第一次更改前创建一个还原点。
fullmove-rebuilding-title = 正在设置你的应用和设置
fullmove-rebuilding-body = 你的文件已就位。PCTwin 正在设置其余部分。请保持这台电脑开机。
fullmove-rebuilding-permission = 正在向这台电脑请求权限
fullmove-rebuilding-apps = 应用：{ $done } / { $total }
fullmove-rebuilding-settings = 设置：{ $done } / { $total }
fullmove-outcome-title = 应用、设置和应用数据
fullmove-outcome-undo = 撤销应用设置
fullmove-outcome-stopped = PCTwin 在中途停止了更改设置，因为它无法记下下一次更改。此后没有任何更改。
fullmove-restore-made = 更改前已创建还原点。
fullmove-restore-earlier-exists = 没有创建新的还原点，因为这台电脑今天已经有一个。
fullmove-restore-not-made = 没有创建还原点。PCTwin 自己的撤销仍然可用。
fullmove-restore-not-windows = 这台电脑没有还原点。PCTwin 自己的撤销仍然可用。
guard-undo-remove-yourself = 如有需要请自行删除
guard-undo-plan-remove-yourself = 你可以在电脑的设置中删除它
guard-undo-result-remove-yourself = 留给你在电脑的设置中删除
guard-undo-plan-keep-other-version = 保留：迁移之后它有了变化
guard-undo-plan-already-gone = 已经删除了
guard-undo-result-kept-other-version = 已保留，因为迁移之后它有了变化
guard-undo-result-removed-restart = 已删除。请重启电脑以完成。
guard-undo-result-not-removed = 没有删除。你可以在电脑的设置中删除它。
guard-undo-close = 完成
move-wait-appdata-title = 正在查看你的应用数据。
move-wait-appdata-body = 已找到你的文件。PCTwin 仍在查看你的应用保存的数据，例如邮件和浏览器。最多需要十分钟。这台电脑上不会做任何更改。
fullmove-special-not-checked = PCTwin 未能完全检查的应用数据
fullmove-group-appdata = PCTwin 将复制的应用数据：{ $count }
fullmove-group-secrets = 你可以选择带过来的机密文件：{ $count }
fullmove-secrets-body = 它们包含密码或密钥。只有你亲自勾选，PCTwin 才会带过来。不勾选就会留在旧电脑上。
fullmove-secret-bring = 带上 { $name }
fullmove-secret-hint = 在这台电脑上只有你能打开它。撤销可以再次删除它。
fullmove-appdata-title = 正在复制你的应用数据
fullmove-appdata-progress = 正在复制应用数据：{ $done } / { $total } 个应用
fullmove-appdata-waiting = 正在等待旧电脑：请在那里关闭 { $apps }。
fullmove-close-title = 关闭这些应用以带上它们的数据
fullmove-close-body = 这些应用在这台电脑上仍然打开，所以 PCTwin 无法安全地复制它们的数据。关闭它们，然后选择重试。如果不带它们继续，它们的数据会留在这里。
fullmove-close-again = 重试
fullmove-close-without = 不带它们继续
fullmove-extras-refused = 无法在这里读取旧电脑的设置和应用数据，所以都没有迁移。它们仍在你的旧电脑上。
fullmove-app-from = PCTwin 会从开发者那里安装 winget 软件包 { $id }，安装前会检查它就是这个应用、且开发者相同。
fullmove-here-older = 这台电脑上有较旧的版本（{ $version }）。PCTwin 保持原样。
fullmove-step-sites = 网站上保存的登录仍与旧电脑绑定。请在这台电脑上重新登录这些网站。
fullmove-links = 你的文件中指向其他位置的快捷方式，PCTwin 不会跟随也不会移动：{ $count }
fullmove-note-sign-in = 打开它并登录你的账户。
fullmove-note-licence-key = 打开它并输入你的许可证密钥，或登录持有你许可证的账户。
fullmove-note-registration = 你的旧电脑显示它注册在 { $who } 名下。
fullmove-note-product-id = 它在旧电脑上的产品 ID 是 { $id }。
fullmove-note-extras = 随它一起来了其他程序：{ $apps }。如果你不需要，请在电脑设置中删除它们。
fullmove-data-lost = 这些数据到达前连接已断开。它们仍安全地保存在你的旧电脑上。开始新的移动即可带过来。
fullmove-data-did-not-arrive = 这些数据没有到达。它们仍安全地保存在你的旧电脑上。开始新的移动即可带过来。
fullmove-data-app-open = 这个应用在旧电脑上是打开的，所以它的数据没有复制。请在那里关闭应用，然后开始新的移动把它带过来。
fullmove-data-changing = PCTwin 读取时，这些数据在旧电脑上一直在变化。请在那里关闭应用，然后开始新的移动把它带过来。
fullmove-data-unreadable = PCTwin 无法读取旧电脑上的这些数据。它们仍在那里。
fullmove-data-not-read = 移动在读取这些数据之前停止了。它们仍安全地保存在你的旧电脑上。
fullmove-data-not-whole = 这些数据没有全部到达，所以 PCTwin 一项都没有放到位。它们仍安全地保存在你的旧电脑上。
fullmove-data-not-landed = 这些数据已到达，但 PCTwin 无法把它们放到这台电脑上的位置。它们仍安全地保存在你的旧电脑上。
fullmove-data-beside = 这个应用在这台电脑上已有自己的文件夹，所以 PCTwin 把旧电脑的副本放在旁边的 { $folder } 中。应用仍使用它自己的文件夹。若要改用旧副本，请关闭应用并互换这两个文件夹。
fullmove-data-placed = 数据在 { $folder } 中，但应用需要先完成一个步骤才能使用。请打开应用检查一下。
fullmove-outcome-here-title = 这台电脑上已有的应用：{ $count }
fullmove-outcome-here = 已在这台电脑上，保持原样。
fullmove-outcome-here-older = 这台电脑上有较旧的版本（{ $version }）。PCTwin 保持了原样。如果想要新版本，请从开发者处更新。
fullmove-outcome-here-badge = 保持原样
fullmove-outcome-fresh = PCTwin 安装的应用会从头开始。请逐个打开，如有要求请登录。
progress-may-sleep = 移动过程中这台电脑可能会进入睡眠。请保持接通电源并打开盖子。

## The old laptop chooses what moves (decided 28 October 2026)
choose-title = 要搬哪些东西？
choose-body = 请在放着你东西的这台笔记本电脑上选择。之后新的笔记本电脑会显示要过来的内容，并请你确认接收。
choose-mode = 要搬什么
choose-everything = 全部
choose-except = 全部，除了
choose-only = 只搬我选的
choose-except-hint = 取消勾选要留下的内容。
choose-open = 打开 { $name }
choose-close = 收起 { $name }
choose-total =
    { $files ->
       *[other] { $files } 个文件
    } · 已选：{ $size }
choose-only-hint = 勾选要带走的内容。
choose-apps = 应用、设置和应用数据
choose-apps-count =
    { $count ->
       *[other] { $count } 个应用，连同它们的设置
    }
choose-group-size =
    { $files ->
       *[other] { $files } 个文件
    } · { $size }
choose-left-title = 这些本来就会留下
choose-left-badge = 留下
choose-left-caches = 缓存和临时文件
choose-left-chosen-by-user = 你选择留下的内容
choose-left-work-managed = 你的工作单位或学校的文件
choose-left-device-bound = 绑定在这台笔记本电脑上的东西，例如通行密钥
choose-left-unreadable = PCTwin 无法读取的文件
choose-left-cloud-only = 只保存在网上的文件
choose-left-system = Windows 自身的部分
choose-left-too-many-files = 太大、一次搬不完的文件夹
choose-left-rebuildable = 新笔记本电脑可以重新生成的东西，例如下载的软件包
choose-nothing = 请至少选择一样要搬的东西。
choose-continue = 继续

## The new laptop accepts what the old laptop chose (decided 28 October 2026)
accept-title = { $from } 想把这些发给你。
accept-body = 这是在你的旧笔记本电脑上选好的。接受后，PCTwin 会把它们搬到这里。
accept-files =
    { $count ->
       *[other] { $count } 个文件
    }
accept-apps =
    { $count ->
       *[other] { $count } 个应用，将为你安装
    }
accept-settings = 设置和应用数据
accept-fits = 这台笔记本电脑放得下。
accept-needs = 有一项需要你先处理。请查看详情。
accept-not-now = 暂不
accept-details = 查看详情
accept-accept = 接受

# 2.8 screens
report-group-apps = 应用
report-group-settings = 设置
report-group-special = 应用数据和其他内容
report-group-people = 一起迁移的其他人
report-count-apps = { $count ->
       *[other] { $count } 个应用
    }
report-count-settings = { $count ->
       *[other] { $count } 组设置
    }
report-count-special = { $count ->
       *[other] { $count } 项
    }
report-count-people = { $count ->
       *[other] { $count } 人
    }
report-beyond-done = 全部完成并已检查
report-beyond-some = { $total } 项中的 { $done } 项已完成并检查
report-beyond-left = { $count ->
       *[other] 按你的选择，{ $count } 项未包含
    }
report-beyond-more = { $count ->
       *[other] 还有 { $count } 个应用、设置或其他内容未在此列表中显示。
    }
report-here = { $count ->
       *[other] { $count } 个应用原本就在这台电脑上。PCTwin 保持了原样。
    }
progress-ready-title = 现在就能用
progress-ready-body = { $count ->
       *[other] { $groups } 已复制并检查。其余内容传输期间，你可以在这台电脑上打开它们。
    }
