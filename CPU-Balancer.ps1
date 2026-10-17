# CPU Balancer — тонкая настройка схем питания процессора ноутбука (Intel hybrid / любой x86).
# Запуск: CPU-Balancer.cmd (сам запросит права администратора).
# -Test : без UI и без изменений, печатает экран для активной схемы (для проверки).
param([switch]$Test, [switch]$Watch)

$ErrorActionPreference = 'Stop'
$AppDir    = Split-Path -Parent $MyInvocation.MyCommand.Path
$BackupDir = Join-Path $AppDir 'backups'

# ---------------------------------------------------------------- настройки Windows
$P = '54533251-82be-4824-96c1-47b60b740d00'   # SUB_PROCESSOR
$S = [ordered]@{
    AUTON       = @($P, '8baa4a8a-14c6-4451-8e8b-14bdbd197537')
    EPP         = @($P, '36687f9e-e3a5-4dbf-b1dc-15eb381c6863')
    EPP1        = @($P, '36687f9e-e3a5-4dbf-b1dc-15eb381c6864')
    BOOST       = @($P, 'be337238-0d82-4146-a960-4f3749d470c7')
    MAXSTATE    = @($P, 'bc5038f7-23e0-4960-96da-33abaf5935ec')
    MINSTATE    = @($P, '893dee8e-2bef-41e0-89c6-b55d0929964c')
    MINSTATE1   = @($P, '893dee8e-2bef-41e0-89c6-b55d0929964d')
    FREQ        = @($P, '75b0ae3f-bce0-45a7-8c89-c9611c25e100')
    FREQ1       = @($P, '75b0ae3f-bce0-45a7-8c89-c9611c25e101')
    CPMIN       = @($P, '0cc5b647-c1df-4637-891a-dec35c318583')
    CPMIN1      = @($P, '0cc5b647-c1df-4637-891a-dec35c318584')
    SCHED       = @($P, '93b8b6dc-0698-4d1c-9ee4-0644e900c85d')
    SHORT       = @($P, 'bae08b81-2d5e-4688-ad6a-13243356654b')
    HETERO      = @($P, '7f2f5cfa-f10c-4823-b5e1-e93ae85f46b5')
    LATPERF     = @($P, '619b7505-003b-4e82-b7a6-4dd29c300971')
    LATUNPARK   = @($P, '616cdaa5-695e-4545-97ad-97dc2d1bdd88')
    FLOORC0     = @($P, 'fddc842b-8364-4edc-94cf-c17f60de1c80')
    INITC1      = @($P, '1facfc65-a930-4bc5-9f38-504ec097bbc0')
    IDLEPROMOTE = @($P, '7b224883-b3cc-4d79-819f-8374152cbe7c')
    IDLESCALE   = @($P, '6c2993b0-8f48-481f-bcc6-00dd2742aa06')
    CONTPOL     = @($P, '60fbe21b-efd9-49f2-b066-8674d8e9f423')
    HYBCU       = @($P, '12fd031f-53d2-4bf4-ac6d-c699fc9538c7')
    HYBCC       = @($P, '6788488b-1b90-4d11-8fa7-973e470dff47')
    EFFCU       = @($P, '6ece9e1f-b6dd-42bf-b1b7-5a512b10c092')
    EFFCC       = @($P, '69439b22-221b-4830-bd34-f7bcece24583')
    CONTINC     = @($P, '64fcee6b-5b1f-45a4-a76a-19b2c36ee290')
    CONTDEC     = @($P, '6ff13aeb-7897-4356-9999-dd9930af065f')
    COOLING     = @($P, '94d3a615-a899-4ac5-ae2b-e4d8f634367f')
    ASPM        = @('501a4d13-42af-4429-9fd1-a8218c268e20', 'ee12f906-d277-404b-b6da-e5fa1a576df5')
}
$GuidToKey = @{}; foreach ($k in $S.Keys) { $GuidToKey[$S[$k][1]] = $k }

$HydrocZone = @{ CONTPOL = 1; HYBCU = 50; HYBCC = 60; EFFCU = 50; EFFCC = 60; CONTINC = 2; CONTDEC = 5 }

# ---------------------------------------------------------------- элементы интерфейса
# slider: Min/Max/Step (←→), Fine (Shift+←→), Big (PgUp/PgDn); enum: Opts = @(значение, подпись)
# mode:   набор настроек сразу; Opts = @{L; Map} или @{L; Backup} (= исходные значения схемы)
$Controls = @(
    @{ T = 'hdr'; Name = 'Главное: скорость <-> экономия' }
    @{ T = 'slider'; Id = 'eppP'; Name = 'EPP P-ядер'; Set = @('EPP1'); Min = 0; Max = 100; Step = 5; Fine = 1; Big = 20; Unit = ''
       Help = 'Главная ручка. Подсказка процессору: 0 = максимум скорости, 100 = максимум экономии. Выше -> ниже частоты, меньше ватт и градусов, медленнее отклик. Обычно 30-65 от сети, 50-85 на батарее. → холоднее' }
    @{ T = 'slider'; Id = 'eppE'; Name = 'EPP E-ядер'; Set = @('EPP'); Min = 0; Max = 100; Step = 5; Fine = 1; Big = 20; Unit = ''
       Help = 'То же для энергоэффективных ядер, на которых живёт фон. Можно держать выше, чем у P-ядер: фон холоднее без потери отклика. → холоднее' }
    @{ T = 'enum'; Id = 'boost'; Name = 'Турбо-буст'; Set = @('BOOST')
       Opts = @(@(2, 'Агрессивный'), @(4, 'Эфф. агрессивный'), @(1, 'Включён'), @(3, 'Эффективный'), @(0, 'Выключен'))
       Help = 'Разрешение подниматься выше базовой частоты. «Выключен» — самый сильный способ снизить нагрев (до −10–20 °C), но процессор заметно медленнее. «Эффективный» включает буст реже. → холоднее' }
    @{ T = 'slider'; Id = 'maxstate'; Name = 'Макс. производительность'; Set = @('MAXSTATE'); Min = 50; Max = 100; Step = 5; Fine = 1; Big = 10; Unit = '%'
       Help = 'Потолок производительности. 100 % = без ограничения. Ниже — резко меньше частота и нагрев; 99 % у многих процессоров отключает турбо-буст. ← холоднее' }
    @{ T = 'slider'; Id = 'freqP'; Name = 'Лимит частоты P-ядер'; Set = @('FREQ1'); Min = 0; Max = 6000; Step = 100; Fine = 100; Big = 500; Unit = 'МГц'; Zero = 'без лимита'
       Help = 'Жёсткий потолок частоты P-ядер, 0 = без лимита. Последние 500–1000 МГц дают мало скорости, но много нагрева — хороший способ остудить ноутбук. ← холоднее' }
    @{ T = 'slider'; Id = 'freqE'; Name = 'Лимит частоты E-ядер'; Set = @('FREQ'); Min = 0; Max = 6000; Step = 100; Fine = 100; Big = 500; Unit = 'МГц'; Zero = 'без лимита'
       Help = 'Потолок частоты E-ядер, 0 = без лимита. На фон влияет сильнее, чем на игры. ← холоднее' }
    @{ T = 'slider'; Id = 'minstate'; Name = 'Мин. производительность'; Set = @('MINSTATE', 'MINSTATE1'); Min = 0; Max = 100; Step = 5; Fine = 1; Big = 20; Unit = '%'
       Help = 'Нижняя граница производительности. Оставь 5 %: выше — процессор не сможет «остыть» в простое. ← холоднее' }

    @{ T = 'hdr'; Name = 'Ядра и распределение задач' }
    @{ T = 'slider'; Id = 'park'; Name = 'Мин. активных ядер'; Set = @('CPMIN', 'CPMIN1'); Min = 0; Max = 100; Step = 5; Fine = 1; Big = 25; Unit = '%'
       Help = 'Сколько ядер всегда держать «разбуженными». 100 % = парковка выключена: отклик ровнее, в простое чуть больше ватт. На батарее лучше низкое значение. ← экономнее' }
    @{ T = 'enum'; Id = 'sched'; Name = 'Куда ставить задачи'; Set = @('SCHED', 'SHORT')
       Opts = @(@(1, 'Только P'), @(2, 'Сначала P'), @(0, 'Любые ядра'), @(5, 'Авто'), @(4, 'Сначала E'), @(3, 'Только E'))
       Help = 'Как Windows делит потоки между P- и E-ядрами. «Авто» — решает Thread Director. «Сначала E» — холоднее и тише, тяжёлое всё равно уходит на P. «Только E» — сильно медленнее. → холоднее' }
    @{ T = 'mode'; Id = 'contain'; Name = 'Фон на E-ядрах'
       Opts = @(@{ L = 'Как было'; Backup = $true }, @{ L = 'Hydroc'; Map = $HydrocZone })
       Help = 'Зоны ограничения Windows 11: лёгкая нагрузка удерживается на E-ядрах и не будит P-ядра. Hydroc — проверенные пороги 50/60 %. Одно из главных средств против перегрева в простое.' }

    @{ T = 'hdr'; Name = 'Отклик, сон и прочее' }
    @{ T = 'mode'; Id = 'latency'; Name = 'Отклик на действия'
       Opts = @(@{ L = 'Быстрый'; Map = @{ LATPERF = 100; LATUNPARK = 50; INITC1 = 100 } }, @{ L = 'Экономный'; Map = @{ LATPERF = 0; LATUNPARK = 0; INITC1 = 0 } }, @{ L = 'Как было'; Backup = $true })
       Help = 'При клике или открытии окна Windows на мгновение поднимает частоту. «Экономный» (как в Hydroc) убирает эти всплески: меньше нагрева и ватт, интерфейс может стать чуть менее резким.' }
    @{ T = 'enum'; Id = 'hwp'; Name = 'Кто управляет частотой'; Set = @('AUTON')
       Opts = @(@(1, 'Процессор (HWP)'), @(0, 'Windows'))
       Help = 'HWP — процессор сам выбирает частоту каждую миллисекунду с учётом EPP. Рекомендуется «Процессор». «Windows» — старый способ, реагирует медленнее, EPP работает хуже.' }
    @{ T = 'enum'; Id = 'aspm'; Name = 'Экономия шины PCIe'; Set = @('ASPM')
       Opts = @(@(0, 'Выкл'), @(1, 'Умеренная'), @(2, 'Максимальная'))
       Help = 'Сон шины PCIe (SSD, Wi-Fi, видеокарта). Экономит 1–3 Вт в простое. Если появятся подвисания или отвал Wi-Fi — верни «Выкл». → экономнее' }
    @{ T = 'enum'; Id = 'cooling'; Name = 'Политика охлаждения'; Set = @('COOLING')
       Opts = @(@(1, 'Активная'), @(0, 'Пассивная'))
       Help = 'Активная — сначала вентиляторы, потом снижение частоты. Пассивная — сначала снижение частоты: тише, но медленнее под нагрузкой. Кривые вентиляторов на ASUS задаёт G-Helper/Armoury.' }
)

# Скрытые настройки, которые пишут только пресеты
$HydrocExtra = @{ FLOORC0 = @(0, 0); IDLEPROMOTE = @(0, 0); IDLESCALE = @(1, 1); HETERO = @(4, 4) }

# ---------------------------------------------------------------- пресеты (значение = @(от сети, от батареи); $null = не трогать)
$Presets = @(
    @{ Name = 'Тихий и холодный'; Desc = 'минимум шума и градусов, фон на E-ядрах'
       Vals = @{ eppP = @(70, 85); eppE = @(80, 90); boost = @(3, 3); maxstate = @(100, 100); sched = @(5, 4); contain = @('Hydroc', 'Hydroc'); latency = @('Экономный', 'Экономный'); hwp = @(1, 1); aspm = @(1, 2); cooling = @(0, 0) }
       Raw = $HydrocExtra }
    @{ Name = 'Баланс (Hydroc)'; Desc = 'логика Hydroc: EPP 64/80, фон на E, без всплесков'
       Vals = @{ eppP = @(64, 80); eppE = @(80, 80); boost = @(2, 3); maxstate = @(100, 100); park = @(100, $null); contain = @('Hydroc', 'Hydroc'); latency = @('Экономный', 'Экономный'); hwp = @(1, 1) }
       Raw = $HydrocExtra }
    @{ Name = 'Hydroc мягче'; Desc = 'Hydroc, но EPP посередине: больше скорости, чуть теплее'
       Vals = @{ eppP = @(48, 65); eppE = @(56, 65); boost = @(2, 3); maxstate = @(100, 100); park = @(100, $null); contain = @('Hydroc', 'Hydroc'); latency = @('Экономный', 'Экономный'); hwp = @(1, 1) }
       Raw = $HydrocExtra }
    @{ Name = 'Производительность'; Desc = 'игры и тяжёлые задачи, быстрый отклик'
       Vals = @{ eppP = @(15, 40); eppE = @(25, 50); boost = @(2, 4); maxstate = @(100, 100); park = @(100, $null); sched = @(5, 5); contain = @('Как было', 'Как было'); latency = @('Быстрый', 'Быстрый'); hwp = @(1, 1); cooling = @(1, $null) }
       Raw = @{ FLOORC0 = @(100, 100); IDLEPROMOTE = @(60, 60); IDLESCALE = @(0, 0) } }
    @{ Name = 'Максимум батареи'; Desc = 'меняет только «от батареи»: буст выкл, EPP 90/95'
       Vals = @{ eppP = @($null, 90); eppE = @($null, 95); boost = @($null, 0); sched = @($null, 4); contain = @($null, 'Hydroc'); latency = @($null, 'Экономный'); hwp = @($null, 1); aspm = @($null, 2); cooling = @($null, 0) }
       Raw = @{} }
)

# ---------------------------------------------------------------- powercfg
function Get-Schemes {
    $list = @()
    foreach ($l in (powercfg /list)) {
        $m = [regex]::Match($l, '([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\s+\((.+?)\)\s*(\*)?\s*$')
        if ($m.Success) { $list += [pscustomobject]@{ Guid = $m.Groups[1].Value; Name = $m.Groups[2].Value; Active = $m.Groups[3].Success } }
    }
    return , $list
}

# Читает все настройки схемы одним вызовом: ключ -> @(AC, DC)
function Read-Scheme($guid) {
    $vals = @{}; $cur = $null; $hex = New-Object System.Collections.Generic.List[long]
    $flush = { if ($cur -and $GuidToKey.ContainsKey($cur) -and $hex.Count -ge 2) { $vals[$GuidToKey[$cur]] = @($hex[$hex.Count - 2], $hex[$hex.Count - 1]) } }
    foreach ($l in (powercfg /qh $guid)) {
        $h = [regex]::Match($l, '0x([0-9a-f]{8})\s*$')
        if ($h.Success) { $hex.Add([Convert]::ToInt64($h.Groups[1].Value, 16)); continue }
        $m = [regex]::Match($l, '([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\s+\(')
        if ($m.Success) { . $flush; $cur = $m.Groups[1].Value; $hex.Clear() }
    }
    . $flush
    return $vals
}

# Исходные значения схемы: backups\<guid>.json. Создаётся при первом открытии;
# если рядом есть Откат_<схема>.cmd от CMD-профилей — берёт исходники оттуда.
function Get-Backup($scheme, $cur) {
    $file = Join-Path $BackupDir "$($scheme.Guid).json"
    $bk = @{}
    if (Test-Path $file) {
        $j = Get-Content $file -Raw -Encoding UTF8 | ConvertFrom-Json
        foreach ($pp in $j.Values.PSObject.Properties) { $bk[$pp.Name] = @([long]$pp.Value[0], [long]$pp.Value[1]) }
    }
    $changed = -not (Test-Path $file)
    $safe = ($scheme.Name -replace '[\\/:*?"<>|]', '_').Trim()
    $rb = Join-Path $AppDir "Откат_$safe.cmd"
    $fromRb = @{}
    if (Test-Path $rb) {
        foreach ($l in (Get-Content $rb)) {
            $m = [regex]::Match($l, 'powercfg /set(ac|dc)valueindex\s+(\S+)\s+\S+\s+(\S+)\s+(\d+)')
            if ($m.Success -and $m.Groups[2].Value -eq $scheme.Guid -and $GuidToKey.ContainsKey($m.Groups[3].Value)) {
                $k = $GuidToKey[$m.Groups[3].Value]
                if (-not $fromRb.ContainsKey($k)) { $fromRb[$k] = @($null, $null) }
                $fromRb[$k][[int]($m.Groups[1].Value -eq 'dc')] = [long]$m.Groups[4].Value
            }
        }
    }
    # Самый ранний бэкап (g16-power-apply.ps1) — приоритетнее отката CMD-профилей
    $old = Join-Path $AppDir 'g16-power-backup.json'
    if (Test-Path $old) {
        foreach ($r in (Get-Content $old -Raw -Encoding UTF8 | ConvertFrom-Json)) {
            if ($r.Scheme -eq $scheme.Guid -and $GuidToKey.ContainsKey($r.Set)) { $fromRb[$GuidToKey[$r.Set]] = @([long]$r.AC, [long]$r.DC) }
        }
    }
    foreach ($k in $cur.Keys) {
        if ($bk.ContainsKey($k)) { continue }
        $v = @($cur[$k][0], $cur[$k][1])
        if ($fromRb.ContainsKey($k)) { for ($c = 0; $c -lt 2; $c++) { if ($null -ne $fromRb[$k][$c]) { $v[$c] = $fromRb[$k][$c] } } }
        $bk[$k] = $v; $changed = $true
    }
    if ($changed -and -not $Test) {
        if (-not (Test-Path $BackupDir)) { New-Item -ItemType Directory $BackupDir | Out-Null }
        $o = [ordered]@{}; foreach ($k in $S.Keys) { if ($bk.ContainsKey($k)) { $o[$k] = $bk[$k] } }
        [pscustomobject]@{ Scheme = $scheme.Guid; Name = $scheme.Name; Values = $o } | ConvertTo-Json -Depth 4 |
            Set-Content -Path $file -Encoding UTF8
    }
    return $bk
}

function Test-OnAC {
    try { $b = Get-CimInstance Win32_Battery -ErrorAction Stop | Select-Object -First 1 } catch { return $true }
    if (-not $b) { return $true }
    return ($b.BatteryStatus -ne 1)
}

# ---------------------------------------------------------------- состояние редактора
$St = @{ Scheme = $null; Cur = @{}; Bk = @{}; Pend = @{}; Row = 1; Col = 0; Msg = ''; OnAC = $true }

function Get-Raw($k, [int]$c) {
    if ($St.Pend.ContainsKey($k)) { return $St.Pend[$k][$c] }
    if ($St.Cur.ContainsKey($k)) { return $St.Cur[$k][$c] }
    return $null
}
function Set-Raw($k, [int]$c, $v) {
    if (-not $St.Cur.ContainsKey($k)) { return }
    if (-not $St.Pend.ContainsKey($k)) { $St.Pend[$k] = @($St.Cur[$k][0], $St.Cur[$k][1]) }
    $St.Pend[$k][$c] = [long]$v
    if ($St.Pend[$k][0] -eq $St.Cur[$k][0] -and $St.Pend[$k][1] -eq $St.Cur[$k][1]) { $St.Pend.Remove($k) }
}

function Get-CtlKeys($ctl) {
    if ($ctl.T -ne 'mode') { return $ctl.Set }
    $ks = @(); foreach ($o in $ctl.Opts) { if ($o.Map) { $ks += $o.Map.Keys } }
    return @($ks | Select-Object -Unique)
}
function Test-Available($ctl) { foreach ($k in (Get-CtlKeys $ctl)) { if (-not $St.Cur.ContainsKey($k)) { return $false } }; return $true }
function Test-Changed($ctl) { foreach ($k in (Get-CtlKeys $ctl)) { if ($St.Pend.ContainsKey($k)) { return $true } }; return $false }

# Индекс варианта mode, совпадающего с текущими значениями (-1 = «своё»)
function Get-ModeIdx($ctl, [int]$c, [switch]$FromBackup) {
    $keys = Get-CtlKeys $ctl
    $get = { param($k) if ($FromBackup) { $St.Bk[$k][$c] } else { Get-Raw $k $c } }
    for ($i = 0; $i -lt $ctl.Opts.Count; $i++) {
        $o = $ctl.Opts[$i]; if (-not $o.Map) { continue }
        $ok = $true; foreach ($k in $o.Map.Keys) { if ((& $get $k) -ne $o.Map[$k]) { $ok = $false; break } }
        if ($ok) { return $i }
    }
    for ($i = 0; $i -lt $ctl.Opts.Count; $i++) {
        if (-not $ctl.Opts[$i].Backup) { continue }
        if ($FromBackup) { return $i }
        $ok = $true; foreach ($k in $keys) { if ((Get-Raw $k $c) -ne $St.Bk[$k][$c]) { $ok = $false; break } }
        if ($ok) { return $i }
    }
    return -1
}
function Set-ModeIdx($ctl, [int]$c, [int]$i) {
    $o = $ctl.Opts[$i]
    if ($o.Map) { foreach ($k in $o.Map.Keys) { Set-Raw $k $c $o.Map[$k] } }
    else { foreach ($k in (Get-CtlKeys $ctl)) { Set-Raw $k $c $St.Bk[$k][$c] } }
}

function Get-CtlText($ctl, [int]$c, [switch]$FromBackup) {
    switch ($ctl.T) {
        'slider' {
            if ($FromBackup) { $v = $St.Bk[$ctl.Set[0]][$c] } else { $v = Get-Raw $ctl.Set[0] $c }
            if ($ctl.Zero -and $v -eq 0) { return $ctl.Zero }
            return ("$v $($ctl.Unit)").Trim()
        }
        'enum' {
            if ($FromBackup) { $v = $St.Bk[$ctl.Set[0]][$c] } else { $v = Get-Raw $ctl.Set[0] $c }
            foreach ($o in $ctl.Opts) { if ($o[0] -eq $v) { return $o[1] } }
            return "своё ($v)"
        }
        'mode' {
            $i = Get-ModeIdx $ctl $c -FromBackup:$FromBackup
            if ($i -lt 0) { return 'своё' }
            return $ctl.Opts[$i].L
        }
    }
}

function Step-Ctl($ctl, [int]$c, [int]$dir, [string]$size) {
    if (-not (Test-Available $ctl)) { return }
    switch ($ctl.T) {
        'slider' {
            $v = Get-Raw $ctl.Set[0] $c
            $stp = $ctl.Step; if ($size -eq 'fine') { $stp = $ctl.Fine } elseif ($size -eq 'big') { $stp = $ctl.Big }
            $n = $v + $dir * $stp
            if ($size -ne 'fine') { $n = [math]::Round($n / $stp) * $stp }     # к сетке шага
            $n = [math]::Max($ctl.Min, [math]::Min($ctl.Max, $n))
            foreach ($k in $ctl.Set) { Set-Raw $k $c $n }
        }
        'enum' {
            $v = Get-Raw $ctl.Set[0] $c
            $i = -1; for ($j = 0; $j -lt $ctl.Opts.Count; $j++) { if ($ctl.Opts[$j][0] -eq $v) { $i = $j } }
            if ($i -lt 0) { $i = 0 } else { $i = [math]::Max(0, [math]::Min($ctl.Opts.Count - 1, $i + $dir)) }
            foreach ($k in $ctl.Set) { Set-Raw $k $c $ctl.Opts[$i][0] }
        }
        'mode' {
            $i = Get-ModeIdx $ctl $c
            if ($i -lt 0) { $i = 0 } else { $i = [math]::Max(0, [math]::Min($ctl.Opts.Count - 1, $i + $dir)) }
            Set-ModeIdx $ctl $c $i
        }
    }
}

function Apply-Preset($pr) {
    foreach ($id in $pr.Vals.Keys) {
        $ctl = $Controls | Where-Object { $_.Id -eq $id } | Select-Object -First 1
        if (-not $ctl -or -not (Test-Available $ctl)) { continue }
        for ($c = 0; $c -lt 2; $c++) {
            $v = $pr.Vals[$id][$c]; if ($null -eq $v) { continue }
            if ($ctl.T -eq 'mode') {
                for ($i = 0; $i -lt $ctl.Opts.Count; $i++) { if ($ctl.Opts[$i].L -eq $v) { Set-ModeIdx $ctl $c $i } }
            } else { foreach ($k in $ctl.Set) { Set-Raw $k $c $v } }
        }
    }
    foreach ($k in $pr.Raw.Keys) { for ($c = 0; $c -lt 2; $c++) { if ($null -ne $pr.Raw[$k][$c]) { Set-Raw $k $c $pr.Raw[$k][$c] } } }
    $St.Msg = "Пресет «$($pr.Name)» подставлен. S — сохранить, Z — отменить правки."
}

function Save-Pending {
    $g = $St.Scheme.Guid; $n = 0; $fail = @()
    foreach ($k in @($St.Pend.Keys)) {
        $sub, $set = $S[$k]
        if ($St.Pend[$k][0] -ne $St.Cur[$k][0]) { powercfg /setacvalueindex $g $sub $set $St.Pend[$k][0]; if ($LASTEXITCODE) { $fail += "$k/AC" } else { $n++ } }
        if ($St.Pend[$k][1] -ne $St.Cur[$k][1]) { powercfg /setdcvalueindex $g $sub $set $St.Pend[$k][1]; if ($LASTEXITCODE) { $fail += "$k/DC" } else { $n++ } }
    }
    if ($St.Scheme.Active) { powercfg /setactive $g | Out-Null }
    $St.Cur = Read-Scheme $g; $St.Pend = @{}
    $St.Msg = "Сохранено значений: $n." + $(if ($fail) { " Не удалось: $($fail -join ', ')" } else { '' }) + $(if (-not $St.Scheme.Active) { ' Схема не активна — A, чтобы включить.' } else { '' })
}

# ---------------------------------------------------------------- ввод: клавиатура + мышь
# ReadConsoleInput даёт и клавиши, и мышь; QuickEdit выключается, иначе клик выделяет текст.
$MouseOK = $false
try {
    Add-Type -ErrorAction Stop -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ConIn {
    [StructLayout(LayoutKind.Sequential)]
    public struct KEY_EVENT_RECORD {
        public int bKeyDown; public ushort wRepeatCount; public ushort wVirtualKeyCode;
        public ushort wVirtualScanCode; public ushort UnicodeChar; public uint dwControlKeyState;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct MOUSE_EVENT_RECORD {
        public short X; public short Y; public uint dwButtonState; public uint dwControlKeyState; public uint dwEventFlags;
    }
    [StructLayout(LayoutKind.Explicit)]
    public struct INPUT_RECORD {
        [FieldOffset(0)] public ushort EventType;
        [FieldOffset(4)] public KEY_EVENT_RECORD Key;
        [FieldOffset(4)] public MOUSE_EVENT_RECORD Mouse;
    }
    [DllImport("kernel32.dll")] static extern IntPtr GetStdHandle(int n);
    [DllImport("kernel32.dll")] static extern bool GetConsoleMode(IntPtr h, out uint m);
    [DllImport("kernel32.dll")] static extern bool SetConsoleMode(IntPtr h, uint m);
    [DllImport("kernel32.dll", EntryPoint = "ReadConsoleInputW")] static extern bool ReadInput(IntPtr h, [Out] INPUT_RECORD[] b, uint len, out uint n);
    [DllImport("kernel32.dll", EntryPoint = "PeekConsoleInputW")] static extern bool PeekInput(IntPtr h, [Out] INPUT_RECORD[] b, uint len, out uint n);
    static IntPtr h; static uint oldMode; static bool on;

    public static bool Enable() {
        h = GetStdHandle(-10);
        if (!GetConsoleMode(h, out oldMode)) return false;
        // + MOUSE_INPUT, EXTENDED_FLAGS, WINDOW_INPUT; − QUICK_EDIT, VIRTUAL_TERMINAL_INPUT
        uint m = (oldMode | 0x0010u | 0x0080u | 0x0008u) & ~(0x0040u | 0x0200u);
        on = SetConsoleMode(h, m);
        return on;
    }
    public static void Restore() { if (on) { SetConsoleMode(h, oldMode); on = false; } }

    // [1, ConsoleKeyInfo] | [2, x, y, buttons, flags, shift] | [3]
    public static object[] Read() {
        var b = new INPUT_RECORD[1]; uint n;
        while (true) {
            if (!ReadInput(h, b, 1, out n) || n == 0) continue;
            var r = b[0];
            if (r.EventType == 1) {
                if (r.Key.bKeyDown == 0) continue;
                uint cs = r.Key.dwControlKeyState;
                var ki = new ConsoleKeyInfo((char)r.Key.UnicodeChar, (ConsoleKey)r.Key.wVirtualKeyCode,
                                            (cs & 0x10) != 0, (cs & 0x03) != 0, (cs & 0x0C) != 0);
                return new object[] { 1, ki };
            }
            if (r.EventType == 2) {
                var m = r.Mouse;
                if (m.dwEventFlags == 1) {
                    if ((m.dwButtonState & 1) == 0) continue;           // просто движение без кнопки — неинтересно
                    var p = new INPUT_RECORD[1]; uint pn;              // из пачки движений берём последнее
                    while (PeekInput(h, p, 1, out pn) && pn == 1 && p[0].EventType == 2 && p[0].Mouse.dwEventFlags == 1) {
                        ReadInput(h, b, 1, out n); m = b[0].Mouse;
                    }
                }
                return new object[] { 2, (int)m.X, (int)m.Y, unchecked((int)m.dwButtonState), (int)m.dwEventFlags, (m.dwControlKeyState & 0x10) != 0 };
            }
            if (r.EventType == 4) return new object[] { 3 };
        }
    }
}
'@
} catch {}

function Read-Input {
    if ($MouseOK) {
        $r = [ConIn]::Read()
        if ($r[0] -eq 1) { return @{ Kind = 'key'; K = $r[1] } }
        if ($r[0] -eq 2) { return @{ Kind = 'mouse'; X = $r[1]; Y = $r[2]; Btn = $r[3]; Flags = $r[4]; Shift = $r[5] } }
        return @{ Kind = 'resize' }
    }
    return @{ Kind = 'key'; K = [Console]::ReadKey($true) }
}

# ---------------------------------------------------------------- отрисовка
# Экран = список строк; строка = список сегментов @(текст, цвет, фон, метка).
# Метка делает сегмент кликабельным: при отрисовке запоминаются его координаты.
$DefaultBack = 'Black'; try { $DefaultBack = [string]$Host.UI.RawUI.BackgroundColor; if ($DefaultBack -eq '-1' -or -not $DefaultBack) { $DefaultBack = 'Black' } } catch {}
function Seg($t, $fg = 'Gray', $bg = $null, $tag = $null) { if (-not $bg) { $bg = $DefaultBack }; return , @([string]$t, $fg, $bg, $tag) }
function Btn($text, $key) { return , (Seg " $text " 'Black' 'DarkCyan' @{ T = 'key'; Key = $key }) }
function Fit($t, $w) { $t = [string]$t; if ($t.Length -gt $w) { return $t.Substring(0, $w - 1) + '…' }; return $t.PadRight($w) }
function Wrap($t, $w) {
    $out = @(); $line = ''
    foreach ($word in ($t -split ' ')) {
        if (($line + ' ' + $word).Trim().Length -gt $w) { $out += $line; $line = $word } else { $line = ($line + ' ' + $word).Trim() }
    }
    if ($line) { $out += $line }
    return , $out
}

# BarW = 21: 20 интервалов -> у EPP ровно 5 единиц на деление
$NameW = 26; $CellW = 36; $BarW = 21

function Get-Frac($ctl, $v) {
    if ($ctl.Zero -and $v -eq 0) { return 1.0 }
    return [math]::Max(0.0, [math]::Min(1.0, ($v - $ctl.Min) / [double]($ctl.Max - $ctl.Min)))
}

function Cell-Segs($ctl, [int]$r, [int]$c, [bool]$sel) {
    $fg = 'White'; $bg = $null
    if ($sel) { $fg = 'Black'; $bg = 'Cyan' }
    $tag = @{ T = 'cell'; Row = $r; Col = $c }
    if (-not (Test-Available $ctl)) { return , @(Seg (Fit 'н/д (нет в системе)' $CellW) 'DarkGray') }
    $txt = Get-CtlText $ctl $c
    if ($ctl.T -eq 'slider') {
        $v = Get-Raw $ctl.Set[0] $c
        $fill = -1; if ((Get-Frac $ctl $v) -gt 0) { $fill = [int][math]::Round((Get-Frac $ctl $v) * ($BarW - 1)) }
        $bpos = [int][math]::Round((Get-Frac $ctl $St.Bk[$ctl.Set[0]][$c]) * ($BarW - 1))
        $bar = ''
        for ($i = 0; $i -lt $BarW; $i++) {
            if ($i -eq $bpos) { if ($i -le $fill) { $bar += '▓' } else { $bar += '│' } }
            elseif ($i -le $fill) { $bar += '█' } else { $bar += '░' }
        }
        $barFg = 'Green'; if ($sel) { $barFg = 'Black' }
        return , @((Seg '[' $fg $bg $tag), (Seg $bar $barFg $bg @{ T = 'bar'; Row = $r; Col = $c }), (Seg (Fit ('] ' + $txt) ($CellW - $BarW - 1)) $fg $bg $tag))
    }
    return , @((Seg '◄ ' $fg $bg @{ T = 'prev'; Row = $r; Col = $c }), (Seg (Fit ($txt + ' ►') ($CellW - 2)) $fg $bg @{ T = 'next'; Row = $r; Col = $c }))
}

function Build-Editor {
    $L = New-Object System.Collections.Generic.List[object]
    $W = 2 + $NameW + 2 * ($CellW + 2) + 2
    $act = ''; if ($St.Scheme.Active) { $act = ' (активна)' }
    $src = 'от батареи'; if ($St.OnAC) { $src = 'от сети' }
    $L.Add(@((Seg ' CPU Balancer ' 'Black' 'Cyan'), (Seg "  Схема: $($St.Scheme.Name)$act" 'White'), (Seg "    Сейчас питание: $src" 'DarkGray')))
    $L.Add(@(Seg ('─' * $W) 'DarkGray'))
    $h0 = 'Gray'; $h1 = 'Gray'; if ($St.OnAC) { $h0 = 'Yellow' } else { $h1 = 'Yellow' }
    if ($St.Col -eq 0) { $h0 = 'Cyan' } else { $h1 = 'Cyan' }
    $L.Add(@((Seg ('  ' + (Fit 'Настройка' $NameW) + '  ') 'Gray'), (Seg (Fit 'ОТ СЕТИ' ($CellW + 2)) $h0 $null @{ T = 'col'; Col = 0 }), (Seg (Fit 'ОТ БАТАРЕИ' $CellW) $h1 $null @{ T = 'col'; Col = 1 })))
    for ($r = 0; $r -lt $Controls.Count; $r++) {
        $ctl = $Controls[$r]
        if ($ctl.T -eq 'hdr') { $L.Add(@(Seg (' ' + $ctl.Name) 'DarkYellow')); continue }
        $sel = ($r -eq $St.Row)
        $mark = '  '; if ($sel) { $mark = '► ' }
        $chg = ' '; if (Test-Changed $ctl) { $chg = '*' }
        $nameFg = 'Gray'; if ($sel) { $nameFg = 'White' }
        $row = @((Seg $mark 'Cyan'), (Seg (Fit $ctl.Name $NameW) $nameFg $null @{ T = 'row'; Row = $r }), (Seg '  '))
        $row += Cell-Segs $ctl $r 0 ($sel -and $St.Col -eq 0)
        $row += , (Seg '  ')
        $row += Cell-Segs $ctl $r 1 ($sel -and $St.Col -eq 1)
        $row += , (Seg " $chg" 'Yellow')
        $L.Add($row)
    }
    $L.Add(@(Seg ('─' * $W) 'DarkGray'))
    $ctl = $Controls[$St.Row]
    $was = ''
    if (Test-Available $ctl) { $was = "Исходно: от сети — $(Get-CtlText $ctl 0 -FromBackup), от батареи — $(Get-CtlText $ctl 1 -FromBackup)" }
    $L.Add(@((Seg " $($ctl.Name). " 'White'), (Seg $was 'DarkGray')))
    $help = Wrap $ctl.Help ($W - 2)
    for ($i = 0; $i -lt 3; $i++) { $ln = ''; if ($i -lt $help.Count) { $ln = $help[$i] }; $L.Add(@(Seg " $ln" 'Gray')) }   # всегда 3 строки — низ не прыгает
    $L.Add(@(Seg ('─' * $W) 'DarkGray'))
    $pl = @(Seg ' Пресеты: ' 'DarkYellow')
    for ($i = 0; $i -lt $Presets.Count; $i++) {
        $tg = @{ T = 'preset'; Idx = $i }
        $pl += , (Seg " $($i + 1) " 'Black' 'DarkYellow' $tg); $pl += , (Seg " $($Presets[$i].Name)" 'Gray' $null $tg); $pl += , (Seg '  ')
    }
    $L.Add($pl)
    $L.Add(@(Seg ''))
    $L.Add(@((Seg ' '), (Btn 'Сохранить  S' 'S'), (Seg ' '), (Btn 'Отменить  Z' 'Z'), (Seg ' '), (Btn 'Исходные  R' 'R'), (Seg ' '),
             (Btn 'Активировать  A' 'A'), (Seg ' '), (Btn 'Копировать  C' 'C'), (Seg ' '), (Btn 'Фон на E-ядрах  F' 'F'), (Seg ' '), (Btn 'Назад  Esc' 'Escape')))
    $L.Add(@(Seg ''))
    if ($MouseOK) { $L.Add(@(Seg ' Мышь: тяни ползунок или кликни по шкале · колесо = шаг (с Shift — точно) · клик по ◄ / по названию ► — переключить' 'DarkGray')) }
    else { $L.Add(@(Seg ' Мышь в этом окне недоступна — управление с клавиатуры.' 'DarkGray')) }
    $L.Add(@(Seg ' Клавиши: ↑↓ выбор · ←→ изменить · Shift+←→ точно · PgUp/PgDn крупно · Tab сеть/батарея · 1-5 пресеты' 'DarkGray'))
    if ($St.Confirm) {
        $L.Add(@((Seg ' Есть несохранённые правки: ' 'Yellow'), (Btn 'Сохранить и выйти  S' 'S'), (Seg ' '), (Btn 'Выйти без сохранения  D' 'D'), (Seg ' '), (Btn 'Остаться' 'Stay')))
    } else {
        $msgFg = 'Green'; if ($St.Msg -match 'Не удалось|ОШИБКА') { $msgFg = 'Red' }
        $L.Add(@(Seg " $($St.Msg)" $msgFg))
    }
    return $L
}

function Build-List($schemes, $sel, $msg) {
    $L = New-Object System.Collections.Generic.List[object]
    $L.Add(@((Seg ' CPU Balancer ' 'Black' 'Cyan'), (Seg '  тонкая настройка процессора ноутбука' 'Gray')))
    $L.Add(@(Seg ('─' * 70) 'DarkGray'))
    $L.Add(@(Seg ' Выбери схему питания:' 'White'))
    for ($i = 0; $i -lt $schemes.Count; $i++) {
        $sc = $schemes[$i]
        $mark = '   '; $fg = 'Gray'; $bg = $null
        if ($i -eq $sel) { $mark = ' ► '; $fg = 'Black'; $bg = 'Cyan' }
        $a = ''; if ($sc.Active) { $a = '  ● активна' }
        $tg = @{ T = 'scheme'; Idx = $i }
        $L.Add(@((Seg $mark 'Cyan' $null $tg), (Seg (Fit $sc.Name 36) $fg $bg $tg), (Seg $a 'Green' $null $tg)))
    }
    $L.Add(@(Seg ('─' * 70) 'DarkGray'))
    $L.Add(@((Seg ' '), (Btn 'Открыть  Enter' 'Enter'), (Seg ' '), (Btn 'Сделать активной  A' 'A'), (Seg ' '), (Btn 'Фоновые программы на E-ядрах  F' 'F'), (Seg ' '), (Btn 'Выход  Esc' 'Escape')))
    $L.Add(@(Seg ''))
    if ($MouseOK) { $L.Add(@(Seg ' Клик — выбрать схему, двойной клик — открыть. Колесо — листать.' 'DarkGray')) }
    $L.Add(@(Seg ' G-Helper / Armoury переключают схемы вместе со своими режимами.' 'DarkGray'))
    $L.Add(@(Seg " $msg" 'Green'))
    return $L
}

# Перерисовываются только изменившиеся строки — перетаскивание без мерцания
$script:Prev = @(); $script:Hot = @()
function Reset-Screen { Clear-Host; $script:Prev = @() }
function Show-Lines($L) {
    $w = 120; try { $w = [Console]::WindowWidth - 1 } catch {}
    $hot = New-Object System.Collections.Generic.List[object]
    $sigs = New-Object System.Collections.Generic.List[string]
    for ($y = 0; $y -lt $L.Count; $y++) {
        $line = $L[$y]; $x = 0; $sig = ''; $parts = @()
        foreach ($sg in $line) {
            $t = $sg[0]; if ($x + $t.Length -gt $w) { $t = $t.Substring(0, [math]::Max(0, $w - $x)) }
            if ($sg[3] -and $t.Length) { $hot.Add(@{ Y = $y; X1 = $x; X2 = $x + $t.Length - 1; Tag = $sg[3] }) }
            $sig += "$t`t$($sg[1])`t$($sg[2])`n"; $parts += , @($t, $sg[1], $sg[2]); $x += $t.Length
        }
        $sigs.Add($sig)
        if ($y -lt $script:Prev.Count -and $script:Prev[$y] -eq $sig) { continue }
        [Console]::SetCursorPosition(0, $y)
        foreach ($pt in $parts) { if ($pt[0].Length) { Write-Host $pt[0] -NoNewline -ForegroundColor $pt[1] -BackgroundColor $pt[2] } }
        Write-Host (' ' * [math]::Max(0, $w - $x)) -NoNewline -BackgroundColor $DefaultBack
    }
    for ($y = $L.Count; $y -lt $script:Prev.Count; $y++) { [Console]::SetCursorPosition(0, $y); Write-Host (' ' * $w) -NoNewline -BackgroundColor $DefaultBack }
    $script:Prev = @($sigs); $script:Hot = $hot
}
function Find-Hot($x, $y) { foreach ($h in $script:Hot) { if ($h.Y -eq $y -and $x -ge $h.X1 -and $x -le $h.X2) { return $h } }; return $null }
function Lines-Text($L) { foreach ($line in $L) { ($line | ForEach-Object { $_[0] }) -join '' } }

# ---------------------------------------------------------------- сценарии
function Open-Scheme($scheme) {
    $St.Scheme = $scheme; $St.Cur = Read-Scheme $scheme.Guid; $St.Bk = Get-Backup $scheme $St.Cur
    $St.Pend = @{}; $St.Msg = 'Правки копятся (*) и применяются кнопкой «Сохранить».'; $St.OnAC = Test-OnAC; $St.Confirm = $false
    if ($Controls[$St.Row].T -eq 'hdr') { $St.Row = 1 }
}

function Move-Row([int]$dir) {
    $r = $St.Row
    do { $r += $dir } while ($r -ge 0 -and $r -lt $Controls.Count -and $Controls[$r].T -eq 'hdr')
    if ($r -ge 0 -and $r -lt $Controls.Count) { $St.Row = $r }
}

# Ползунок по координате мыши внутри шкалы
function Set-SliderX($ctl, [int]$c, [int]$x, [int]$x1, [int]$x2) {
    if (-not (Test-Available $ctl)) { return }
    $fr = [math]::Max(0.0, [math]::Min(1.0, ($x - $x1) / [double]($x2 - $x1)))
    if ($ctl.Zero -and $fr -ge 0.999) { $v = 0 }
    else {
        $v = $ctl.Min + $fr * ($ctl.Max - $ctl.Min)
        $v = [math]::Round($v / $ctl.Step) * $ctl.Step
        $v = [math]::Max($ctl.Min, [math]::Min($ctl.Max, $v))
    }
    foreach ($k in $ctl.Set) { Set-Raw $k $c $v }
}

# Переключатель по кругу (для клика мышью)
function Cycle-Ctl($ctl, [int]$c, [int]$dir) {
    if (-not (Test-Available $ctl)) { return }
    $n = $ctl.Opts.Count
    if ($ctl.T -eq 'enum') {
        $v = Get-Raw $ctl.Set[0] $c; $i = -1
        for ($j = 0; $j -lt $n; $j++) { if ($ctl.Opts[$j][0] -eq $v) { $i = $j } }
        $i = (($i + $dir) % $n + $n) % $n
        foreach ($k in $ctl.Set) { Set-Raw $k $c $ctl.Opts[$i][0] }
    } elseif ($ctl.T -eq 'mode') {
        $i = Get-ModeIdx $ctl $c
        $i = (($i + $dir) % $n + $n) % $n
        Set-ModeIdx $ctl $c $i
    }
}

# Возвращает $true, если нужно выйти из редактора
function Editor-Key([string]$key, [bool]$shift, [char]$ch) {
    $ctl = $Controls[$St.Row]
    $St.Msg = ''
    switch ($key) {
        'UpArrow'    { Move-Row -1 }
        'DownArrow'  { Move-Row 1 }
        'LeftArrow'  { if ($shift) { Step-Ctl $ctl $St.Col -1 'fine' } else { Step-Ctl $ctl $St.Col -1 'step' } }
        'RightArrow' { if ($shift) { Step-Ctl $ctl $St.Col 1 'fine' } else { Step-Ctl $ctl $St.Col 1 'step' } }
        'PageDown'   { Step-Ctl $ctl $St.Col -1 'big' }
        'PageUp'     { Step-Ctl $ctl $St.Col 1 'big' }
        'Tab'        { $St.Col = 1 - $St.Col }
        'C' {
            $o = 1 - $St.Col
            if ($ctl.T -eq 'mode') { $i = Get-ModeIdx $ctl $St.Col; if ($i -ge 0) { Set-ModeIdx $ctl $o $i } }
            else { foreach ($kk in $ctl.Set) { Set-Raw $kk $o (Get-Raw $kk $St.Col) } }
            $St.Msg = 'Значение скопировано в соседний столбец.'
        }
        'S' { if ($St.Pend.Count) { Save-Pending } else { $St.Msg = 'Нечего сохранять.' } }
        'Z' { $St.Pend = @{}; $St.Msg = 'Несохранённые правки отменены.' }
        'R' { foreach ($kk in $St.Bk.Keys) { for ($c = 0; $c -lt 2; $c++) { Set-Raw $kk $c $St.Bk[$kk][$c] } }; $St.Msg = 'Исходные значения подставлены. «Сохранить» — применить.' }
        'A' { powercfg /setactive $St.Scheme.Guid | Out-Null; $St.Scheme.Active = $true; $St.Msg = 'Схема сделана активной.' }
        'F' { Run-Eco; Reset-Screen }
        'Escape' {
            if (-not $St.Pend.Count) { return $true }
            $St.Confirm = $true
            try {
                while ($true) {
                    Show-Lines (Build-Editor)
                    $ev = Read-Input
                    $ans = $null
                    if ($ev.Kind -eq 'key') { $ans = [string]$ev.K.Key; if ($ans -ne 'S' -and $ans -ne 'D') { $ans = 'Stay' } }
                    elseif ($ev.Kind -eq 'mouse' -and ($ev.Btn -band 1) -and $ev.Flags -eq 0) {
                        $h = Find-Hot $ev.X $ev.Y; $ans = 'Stay'; if ($h -and $h.Tag.T -eq 'key') { $ans = $h.Tag.Key }
                    }
                    elseif ($ev.Kind -eq 'resize') { Reset-Screen }
                    if ($ans -eq 'S') { Save-Pending; return $true }
                    if ($ans -eq 'D') { $St.Pend = @{}; return $true }
                    if ($ans -eq 'Stay') { return $false }
                }
            } finally { $St.Confirm = $false }
        }
        default {
            $d = [int]$ch - [int][char]'1'
            if ($d -ge 0 -and $d -lt $Presets.Count) { Apply-Preset $Presets[$d] }
        }
    }
    return $false
}

$script:Drag = $null
function Editor-Mouse($ev) {
    $h = Find-Hot $ev.X $ev.Y
    $tag = $null; if ($h) { $tag = $h.Tag }
    $left = [bool]($ev.Btn -band 1)

    if ($ev.Flags -band 4) {                                   # колесо
        $dir = 1; if ($ev.Btn -lt 0) { $dir = -1 }
        if ($tag -and $tag.ContainsKey('Row')) {
            $St.Row = $tag.Row; if ($tag.ContainsKey('Col')) { $St.Col = $tag.Col }
            $size = 'step'; if ($ev.Shift) { $size = 'fine' }
            Step-Ctl $Controls[$St.Row] $St.Col $dir $size
        } else { Move-Row (-$dir) }
        return $false
    }
    if ($ev.Flags -eq 1) {                                     # перетаскивание
        if ($left -and $script:Drag) {
            $dr = $script:Drag
            Set-SliderX $Controls[$dr.Row] $dr.Col $ev.X $dr.X1 $dr.X2
        }
        return $false
    }
    if (-not $left) { $script:Drag = $null; return $false }     # кнопку отпустили
    if (-not $tag) { return $false }

    if ($tag.ContainsKey('Row')) { $St.Row = $tag.Row; $St.Msg = '' }
    if ($tag.ContainsKey('Col')) { $St.Col = $tag.Col }
    switch ($tag.T) {
        'bar'    { Set-SliderX $Controls[$tag.Row] $tag.Col $ev.X $h.X1 $h.X2; $script:Drag = @{ Row = $tag.Row; Col = $tag.Col; X1 = $h.X1; X2 = $h.X2 } }
        'prev'   { Cycle-Ctl $Controls[$tag.Row] $tag.Col -1 }
        'next'   { Cycle-Ctl $Controls[$tag.Row] $tag.Col 1 }
        'preset' { Apply-Preset $Presets[$tag.Idx] }
        'key'    { return (Editor-Key $tag.Key $false ([char]0)) }
    }
    return $false
}

function Run-Editor {
    Reset-Screen; $script:Drag = $null
    while ($true) {
        Show-Lines (Build-Editor)
        $ev = Read-Input
        $exit = $false
        if ($ev.Kind -eq 'key') {
            $k = $ev.K
            $exit = Editor-Key ([string]$k.Key) ([bool]($k.Modifiers -band [ConsoleModifiers]::Shift)) $k.KeyChar
        } elseif ($ev.Kind -eq 'mouse') {
            $exit = Editor-Mouse $ev
        } else { Reset-Screen }
        if ($exit) { return }
    }
}

function Run-List {
    $sel = 0; $msg = ''
    while ($true) {
        $schemes = Get-Schemes
        if ($sel -ge $schemes.Count) { $sel = 0 }
        Reset-Screen
        $reload = $false
        while (-not $reload) {
            Show-Lines (Build-List $schemes $sel $msg); $msg = ''
            $ev = Read-Input
            $key = ''
            if ($ev.Kind -eq 'resize') { Reset-Screen; continue }
            if ($ev.Kind -eq 'key') { $key = [string]$ev.K.Key }
            elseif ($ev.Flags -band 4) { if ($ev.Btn -lt 0) { $key = 'DownArrow' } else { $key = 'UpArrow' } }
            elseif (($ev.Btn -band 1) -and ($ev.Flags -eq 0 -or $ev.Flags -eq 2)) {
                $h = Find-Hot $ev.X $ev.Y
                if ($h -and $h.Tag.T -eq 'scheme') { $sel = $h.Tag.Idx; if ($ev.Flags -eq 2) { $key = 'Enter' } }
                elseif ($h -and $h.Tag.T -eq 'key') { $key = $h.Tag.Key }
            }
            switch ($key) {
                'UpArrow'   { $sel = [math]::Max(0, $sel - 1) }
                'DownArrow' { $sel = [math]::Min($schemes.Count - 1, $sel + 1) }
                'Escape'    { return }
                'A'         { powercfg /setactive $schemes[$sel].Guid | Out-Null; $msg = "Активна: $($schemes[$sel].Name)"; $reload = $true }
                'Enter'     { Open-Scheme $schemes[$sel]; Run-Editor; $reload = $true }
                'F'         { Run-Eco; $reload = $true }
            }
        }
    }
}

# ---------------------------------------------------------------- фоновые программы на E-ядрах
# EcoQoS — тот же механизм, что «Режим эффективности» в Диспетчере задач: Windows держит
# процесс на E-ядрах на низкой частоте. «Только E-ядра» дополнительно жёстко привязывает его к ним.
# Режим живёт, пока жив процесс, поэтому есть наблюдатель (-Watch) в Планировщике заданий.
$EcoConfig   = Join-Path $AppDir 'eco-apps.json'
$EcoTaskName = 'CPU Balancer - фон на E-ядрах'
$EcoModes    = @('Обычный', 'Эко', 'Только E-ядра')
$EcoSkip     = @('idle', 'system', 'registry', 'smss', 'csrss', 'wininit', 'services', 'lsass', 'lsaiso', 'winlogon',
                 'fontdrvhost', 'dwm', 'memory compression', 'secure system', 'audiodg', 'conhost', 'svchost', 'taskmgr')

try {
    Add-Type -ErrorAction Stop -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class Eco {
    [StructLayout(LayoutKind.Sequential)]
    struct THROTTLE { public uint Version; public uint ControlMask; public uint StateMask; }
    [DllImport("kernel32.dll")] static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);
    [DllImport("kernel32.dll")] static extern bool SetProcessInformation(IntPtr h, int cls, ref THROTTLE info, int size);
    [DllImport("kernel32.dll")] static extern bool GetProcessInformation(IntPtr h, int cls, ref THROTTLE info, int size);
    [DllImport("kernel32.dll")] static extern bool SetPriorityClass(IntPtr h, uint cls);
    [DllImport("kernel32.dll")] static extern bool GetProcessAffinityMask(IntPtr h, out UIntPtr proc, out UIntPtr sys);
    [DllImport("kernel32.dll")] static extern bool SetProcessAffinityMask(IntPtr h, UIntPtr mask);
    [DllImport("kernel32.dll")] static extern bool GetSystemCpuSetInformation(IntPtr info, int len, out int ret, IntPtr proc, int flags);

    const uint SET_INFO = 0x0200, QUERY_LIMITED = 0x1000;
    const int  POWER_THROTTLING = 4;                 // ProcessPowerThrottling
    const uint EXEC_SPEED = 0x1, IGNORE_TIMER = 0x4; // EcoQoS + не держать высокое разрешение таймера

    // Логические процессоры самого экономного класса (E + LP-E). 0 — если ядра одинаковые.
    public static ulong EfficientMask() {
        int len; GetSystemCpuSetInformation(IntPtr.Zero, 0, out len, IntPtr.Zero, 0);
        if (len <= 0) return 0;
        IntPtr buf = Marshal.AllocHGlobal(len);
        try {
            if (!GetSystemCpuSetInformation(buf, len, out len, IntPtr.Zero, 0)) return 0;
            int min = 255, max = 0; var lp = new System.Collections.Generic.List<int[]>();
            for (int off = 0; off < len; ) {
                int size = Marshal.ReadInt32(buf, off);
                int group = Marshal.ReadInt16(buf, off + 12), idx = Marshal.ReadByte(buf, off + 14), cls = Marshal.ReadByte(buf, off + 18);
                if (group == 0 && idx < 64) { lp.Add(new[] { idx, cls }); min = Math.Min(min, cls); max = Math.Max(max, cls); }
                off += size;
            }
            if (min == max) return 0;
            ulong m = 0; foreach (var x in lp) if (x[1] == min) m |= 1UL << x[0];
            return m;
        } finally { Marshal.FreeHGlobal(buf); }
    }

    // mode: 0 — вернуть как было, 1 — EcoQoS, 2 — EcoQoS + только E-ядра. Возвращает false, если доступ запрещён.
    public static bool Apply(int pid, int mode, ulong eMask) {
        IntPtr h = OpenProcess(SET_INFO | QUERY_LIMITED, false, pid);
        if (h == IntPtr.Zero) return false;
        try {
            var t = new THROTTLE { Version = 1 };
            if (mode > 0) { t.ControlMask = EXEC_SPEED | IGNORE_TIMER; t.StateMask = EXEC_SPEED | IGNORE_TIMER; }
            bool ok = SetProcessInformation(h, POWER_THROTTLING, ref t, Marshal.SizeOf(typeof(THROTTLE)));
            SetPriorityClass(h, mode > 0 ? 0x4000u : 0x20u);   // BELOW_NORMAL / NORMAL
            UIntPtr pm, sm;
            if (GetProcessAffinityMask(h, out pm, out sm)) {
                ulong all = sm.ToUInt64();
                ulong want = (mode == 2 && (eMask & all) != 0) ? (eMask & all) : all;
                if (pm.ToUInt64() != want) SetProcessAffinityMask(h, new UIntPtr(want));
            }
            return ok;
        } finally { CloseHandle(h); }
    }

    // Текущее состояние: -1 нет доступа, 0 обычный, 1 EcoQoS, 2 EcoQoS + урезанная привязка
    public static int State(int pid) {
        IntPtr h = OpenProcess(QUERY_LIMITED, false, pid);
        if (h == IntPtr.Zero) return -1;
        try {
            var t = new THROTTLE { Version = 1 };
            if (!GetProcessInformation(h, POWER_THROTTLING, ref t, Marshal.SizeOf(typeof(THROTTLE)))) return -1;
            if ((t.StateMask & EXEC_SPEED) == 0) return 0;
            UIntPtr pm, sm;
            if (GetProcessAffinityMask(h, out pm, out sm) && pm.ToUInt64() != sm.ToUInt64()) return 2;
            return 1;
        } finally { CloseHandle(h); }
    }
}
'@
} catch {}
$EMask = [uint64]0; try { $EMask = [Eco]::EfficientMask() } catch {}

function Get-EcoApps {
    $apps = @{}
    if (Test-Path $EcoConfig) {
        try {
            $j = Get-Content $EcoConfig -Raw -Encoding UTF8 | ConvertFrom-Json
            foreach ($pp in $j.Apps.PSObject.Properties) { $apps[$pp.Name.ToLower()] = [int]$pp.Value }
        } catch {}
    }
    return $apps
}
function Save-EcoApps($apps) {
    $o = [ordered]@{}; foreach ($n in ($apps.Keys | Sort-Object)) { if ($apps[$n] -gt 0) { $o[$n] = $apps[$n] } }
    [pscustomobject]@{ Apps = $o } | ConvertTo-Json -Depth 3 | Set-Content -Path $EcoConfig -Encoding UTF8
}

# Применить режим ко всем запущенным процессам с этим exe. Возвращает @(успешно, отказано)
function Apply-EcoName($exe, [int]$mode) {
    $ok = 0; $bad = 0
    foreach ($pr in (Get-Process -Name ([IO.Path]::GetFileNameWithoutExtension($exe)) -ErrorAction SilentlyContinue)) {
        if ($pr.Id -eq $PID) { continue }
        if ([Eco]::Apply($pr.Id, $mode, $EMask)) { $ok++ } else { $bad++ }
    }
    return , @($ok, $bad)
}

function Get-EcoTask {
    try { return Get-ScheduledTask -TaskName $EcoTaskName -ErrorAction Stop } catch { return $null }
}
function Enable-EcoTask {
    $self = Join-Path $AppDir 'CPU-Balancer.ps1'
    # conhost --headless — без мелькающего окна консоли
    $act = New-ScheduledTaskAction -Execute 'conhost.exe' -Argument "--headless powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$self`" -Watch"
    $trg = New-ScheduledTaskTrigger -AtLogOn -User "$env:USERDOMAIN\$env:USERNAME"
    $prn = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive -RunLevel Highest
    $set = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew
    Register-ScheduledTask -TaskName $EcoTaskName -Action $act -Trigger $trg -Principal $prn -Settings $set -Force | Out-Null
    Start-ScheduledTask -TaskName $EcoTaskName
}
function Disable-EcoTask {
    try { Stop-ScheduledTask -TaskName $EcoTaskName -ErrorAction Stop } catch {}
    try { Unregister-ScheduledTask -TaskName $EcoTaskName -Confirm:$false -ErrorAction Stop } catch {}
}

# Наблюдатель: применяет режимы к новым процессам сразу при запуске + проверка раз в 30 с
function Run-Watch {
    $apps = @{}; $stamp = $null
    $reload = {
        $t = $null; if (Test-Path $EcoConfig) { $t = (Get-Item $EcoConfig).LastWriteTimeUtc }
        if ($t -ne $stamp) { $script:WatchApps = Get-EcoApps; $stamp = $t }
    }
    $sweep = {
        . $reload
        foreach ($n in $script:WatchApps.Keys) {
            $m = $script:WatchApps[$n]; if ($m -le 0) { continue }
            foreach ($pr in (Get-Process -Name ([IO.Path]::GetFileNameWithoutExtension($n)) -ErrorAction SilentlyContinue)) {
                if ([Eco]::State($pr.Id) -lt $m) { [void][Eco]::Apply($pr.Id, $m, $EMask) }
            }
        }
    }
    $script:WatchApps = @{}
    . $sweep
    $events = $true
    try { Register-CimIndicationEvent -ClassName Win32_ProcessStartTrace -SourceIdentifier EcoStart -ErrorAction Stop | Out-Null } catch { $events = $false }
    while ($true) {
        if (-not $events) { Start-Sleep -Seconds 5; . $sweep; continue }
        $e = Wait-Event -SourceIdentifier EcoStart -Timeout 30
        if (-not $e) { . $sweep; continue }
        $ne = $e.SourceEventArgs.NewEvent
        Remove-Event -EventIdentifier $e.EventIdentifier
        . $reload
        $n = ([string]$ne.ProcessName).ToLower()
        if ($script:WatchApps.ContainsKey($n) -and $script:WatchApps[$n] -gt 0) { [void][Eco]::Apply([int]$ne.ProcessID, $script:WatchApps[$n], $EMask) }
    }
}

# Список для экрана: выбранные программы + запущенные, с загрузкой CPU за ~0,6 с
function Get-EcoRows($apps) {
    $snap = @{}
    foreach ($pr in (Get-Process -ErrorAction SilentlyContinue)) { try { $snap[$pr.Id] = $pr.TotalProcessorTime.TotalMilliseconds } catch {} }
    Start-Sleep -Milliseconds 600
    $by = @{}
    foreach ($pr in (Get-Process -ErrorAction SilentlyContinue)) {
        if ($pr.Id -eq $PID -or $EcoSkip -contains $pr.ProcessName.ToLower()) { continue }
        $n = $pr.ProcessName.ToLower() + '.exe'
        if (-not $by.ContainsKey($n)) { $by[$n] = @{ N = 0; Cpu = 0.0; Eco = 0 } }
        $r = $by[$n]; $r.N++
        try { if ($snap.ContainsKey($pr.Id)) { $r.Cpu += ($pr.TotalProcessorTime.TotalMilliseconds - $snap[$pr.Id]) } } catch {}
        if ([Eco]::State($pr.Id) -gt 0) { $r.Eco++ }
    }
    $ncpu = [Environment]::ProcessorCount
    $rows = @()
    foreach ($n in $by.Keys) { $rows += [pscustomobject]@{ Name = $n; Procs = $by[$n].N; Cpu = [math]::Round($by[$n].Cpu / (6 * $ncpu), 1); Eco = $by[$n].Eco } }
    foreach ($n in $apps.Keys) { if (-not $by.ContainsKey($n)) { $rows += [pscustomobject]@{ Name = $n; Procs = 0; Cpu = 0; Eco = 0 } } }
    $sel = @($rows | Where-Object { $apps.ContainsKey($_.Name) } | Sort-Object Name)
    $rest = @($rows | Where-Object { -not $apps.ContainsKey($_.Name) } | Sort-Object @{ e = 'Cpu'; Descending = $true }, Name)
    return , @($sel + $rest)
}

function Build-Eco($E) {
    $L = New-Object System.Collections.Generic.List[object]
    $W = 100
    $L.Add(@((Seg ' CPU Balancer ' 'Black' 'Cyan'), (Seg '  Фоновые программы на E-ядрах' 'White')))
    $L.Add(@(Seg ('─' * $W) 'DarkGray'))
    $nE = 0; for ($i = 0; $i -lt 64; $i++) { if ($EMask -band ([uint64]1 -shl $i)) { $nE++ } }
    $cores = 'ядра одного типа — «Только E-ядра» недоступно'
    if ($nE) { $cores = "экономных потоков: $nE из $([Environment]::ProcessorCount)" }
    $task = Get-EcoTask
    $auto = @(Seg '○ выключено' 'DarkGray')
    if ($task) { $tst = 'ждёт входа в систему'; if ([string]$task.State -eq 'Running') { $tst = 'работает' }; $auto = @(Seg "● включено ($tst)" 'Green') }
    $L.Add(@((Seg " $cores" 'Gray'), (Seg '      Автоприменение: ' 'Gray')) + $auto)
    $L.Add(@(Seg ''))
    $L.Add(@(Seg ('  ' + (Fit 'Программа' 34) + (Fit 'Режим' 22) + (Fit 'Процессов' 11) + (Fit 'CPU сейчас' 11) + 'Сейчас в эко') 'Gray'))
    $wh = 40; try { $wh = [Console]::WindowHeight } catch {}; $visible = [math]::Max(5, $wh - 17)
    if ($E.Row -lt $E.Top) { $E.Top = $E.Row }
    if ($E.Row -ge $E.Top + $visible) { $E.Top = $E.Row - $visible + 1 }
    for ($i = $E.Top; $i -lt [math]::Min($E.Rows.Count, $E.Top + $visible); $i++) {
        $r = $E.Rows[$i]; $sel = ($i -eq $E.Row)
        $m = 0; if ($E.Apps.ContainsKey($r.Name)) { $m = $E.Apps[$r.Name] }
        $fg = 'Gray'; if ($m -gt 0) { $fg = 'White' }
        $cf = 'White'; $cb = $null; if ($sel) { $cf = 'Black'; $cb = 'Cyan' }
        $mark = '  '; if ($sel) { $mark = '► ' }
        $mfg = $cf; if (-not $sel -and $m -eq 1) { $mfg = 'Green' } elseif (-not $sel -and $m -eq 2) { $mfg = 'Yellow' }
        $tg = @{ T = 'erow'; Idx = $i }
        $eco = ''; if ($r.Procs) { $eco = "$($r.Eco) из $($r.Procs)" }
        $L.Add(@((Seg $mark 'Cyan' $null $tg), (Seg (Fit $r.Name 34) $fg $null $tg),
                 (Seg '◄ ' $mfg $cb @{ T = 'eprev'; Idx = $i }), (Seg (Fit ($EcoModes[$m] + ' ►') 20) $mfg $cb @{ T = 'enext'; Idx = $i }),
                 (Seg (Fit $r.Procs 11) 'Gray' $null $tg), (Seg (Fit ("$($r.Cpu) %") 11) 'Gray' $null $tg), (Seg $eco 'DarkGray' $null $tg)))
    }
    if ($E.Rows.Count -gt $visible) { $L.Add(@(Seg ("  … показано $($E.Top + 1)–$([math]::Min($E.Rows.Count, $E.Top + $visible)) из $($E.Rows.Count), колесо/↑↓ — листать") 'DarkGray')) }
    $L.Add(@(Seg ('─' * $W) 'DarkGray'))
    foreach ($ln in (Wrap 'Эко — Windows считает программу фоновой (EcoQoS, как «Режим эффективности» в Диспетчере задач): она работает на E-ядрах на низкой частоте и не будит P-ядра. Только E-ядра — ещё и жёсткая привязка к экономным ядрам: максимум холода, но программа медленнее. Не включай для игр и того, чем пользуешься активно.' ($W - 2))) { $L.Add(@(Seg " $ln" 'Gray')) }
    $L.Add(@(Seg ''))
    $tw = 'Включить автоприменение  W'; if ($task) { $tw = 'Выключить автоприменение  W' }
    $L.Add(@((Seg ' '), (Btn $tw 'W'), (Seg ' '), (Btn 'Обновить список  U' 'U'), (Seg ' '), (Btn 'Назад  Esc' 'Escape')))
    $L.Add(@(Seg ' Режим действует, пока программа запущена. Автоприменение (Планировщик заданий) включает его заново при каждом запуске.' 'DarkGray'))
    $msgFg = 'Green'; if ($E.Msg -match 'Не удалось|отказ|ОШИБКА') { $msgFg = 'Yellow' }
    $L.Add(@(Seg " $($E.Msg)" $msgFg))
    return $L
}

function Set-EcoMode($E, [int]$i, [int]$dir) {
    if ($i -lt 0 -or $i -ge $E.Rows.Count) { return }
    $n = $E.Rows[$i].Name
    $m = 0; if ($E.Apps.ContainsKey($n)) { $m = $E.Apps[$n] }
    $max = 2; if (-not $EMask) { $max = 1 }
    $m = (($m + $dir) % ($max + 1) + ($max + 1)) % ($max + 1)
    if ($m -gt 0) { $E.Apps[$n] = $m } else { $E.Apps.Remove($n) }
    Save-EcoApps $E.Apps
    $res = Apply-EcoName $n $m
    $E.Rows[$i].Eco = 0; if ($m -gt 0) { $E.Rows[$i].Eco = $res[0] }
    $E.Msg = "$n → $($EcoModes[$m])."
    if ($res[0] + $res[1]) { $E.Msg += " Применено к $($res[0]) из $($res[0] + $res[1]) процессов." }
    if ($res[1]) { $E.Msg += ' Часть процессов защищена системой — отказ доступа.' }
    if ($m -gt 0 -and -not (Get-EcoTask)) { $E.Msg += ' Включи автоприменение (W), чтобы режим сохранялся после перезапуска программы.' }
}

function Run-Eco {
    $E = @{ Apps = (Get-EcoApps); Rows = @(); Row = 0; Top = 0; Msg = 'Считаю загрузку…' }
    Reset-Screen; Show-Lines (Build-Eco $E)
    $E.Rows = Get-EcoRows $E.Apps; $E.Msg = '←→ или клик по режиму — переключить. Сверху — уже выбранные, ниже — запущенные по загрузке CPU.'
    while ($true) {
        Show-Lines (Build-Eco $E)
        $ev = Read-Input
        $key = ''
        if ($ev.Kind -eq 'resize') { Reset-Screen; continue }
        if ($ev.Kind -eq 'key') { $key = [string]$ev.K.Key }
        elseif ($ev.Flags -band 4) { if ($ev.Btn -lt 0) { $key = 'DownArrow' } else { $key = 'UpArrow' } }
        elseif (($ev.Btn -band 1) -and ($ev.Flags -eq 0 -or $ev.Flags -eq 2)) {
            $h = Find-Hot $ev.X $ev.Y
            if ($h) {
                switch ($h.Tag.T) {
                    'erow'  { $E.Row = $h.Tag.Idx }
                    'eprev' { $E.Row = $h.Tag.Idx; Set-EcoMode $E $E.Row -1 }
                    'enext' { $E.Row = $h.Tag.Idx; Set-EcoMode $E $E.Row 1 }
                    'key'   { $key = $h.Tag.Key }
                }
            }
        }
        switch ($key) {
            'UpArrow'    { $E.Row = [math]::Max(0, $E.Row - 1) }
            'DownArrow'  { $E.Row = [math]::Min($E.Rows.Count - 1, $E.Row + 1) }
            'PageUp'     { $E.Row = [math]::Max(0, $E.Row - 10) }
            'PageDown'   { $E.Row = [math]::Min($E.Rows.Count - 1, $E.Row + 10) }
            'LeftArrow'  { Set-EcoMode $E $E.Row -1 }
            'RightArrow' { Set-EcoMode $E $E.Row 1 }
            'U'          { $E.Msg = 'Считаю загрузку…'; Show-Lines (Build-Eco $E); $E.Rows = Get-EcoRows $E.Apps; $E.Row = [math]::Min($E.Row, [math]::Max(0, $E.Rows.Count - 1)); $E.Msg = 'Список обновлён.' }
            'W' {
                try {
                    if (Get-EcoTask) { Disable-EcoTask; $E.Msg = 'Автоприменение выключено. Уже применённый режим останется до перезапуска программ.' }
                    else { Enable-EcoTask; $E.Msg = 'Автоприменение включено: работает сейчас и при каждом входе в систему.' }
                } catch { $E.Msg = "ОШИБКА Планировщика заданий: $($_.Exception.Message)" }
            }
            'Escape'     { return }
        }
    }
}

# ---------------------------------------------------------------- запуск
if ($Test) {
    $sch = (Get-Schemes) | Where-Object Active | Select-Object -First 1
    Open-Scheme $sch
    Lines-Text (Build-Editor)
    ''
    'Прочитано настроек: ' + $St.Cur.Count + ' из ' + $S.Count
    $miss = @($S.Keys | Where-Object { -not $St.Cur.ContainsKey($_) }); if ($miss) { 'Нет в системе: ' + ($miss -join ', ') }
    'Исходные (backup) EPP1: ' + ($St.Bk['EPP1'] -join '/') + '  EPP: ' + ($St.Bk['EPP'] -join '/') + '  BOOST: ' + ($St.Bk['BOOST'] -join '/')
    Apply-Preset $Presets[2]
    'После пресета «Hydroc мягче» (не сохранено): ' + (($St.Pend.Keys | Sort-Object | ForEach-Object { "$_=" + ($St.Pend[$_] -join '/') }) -join '  ')
    ''
    $EcoTest = @{ Apps = (Get-EcoApps); Rows = (Get-EcoRows (Get-EcoApps)); Row = 0; Top = 0; Msg = '' }
    Lines-Text (Build-Eco $EcoTest) | Select-Object -First 14
    'Маска E-ядер: 0x' + $EMask.ToString('X')
    return
}

if ($Watch) { Run-Watch; return }

$me = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $me.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Start-Process powershell.exe -Verb RunAs -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$($MyInvocation.MyCommand.Path)`""
    exit
}

try {
    $Host.UI.RawUI.WindowTitle = 'CPU Balancer'
    try { if ([Console]::WindowWidth -lt 115) { [Console]::WindowWidth = [math]::Min(115, [Console]::LargestWindowWidth) } } catch {}
    try { if ([Console]::WindowHeight -lt 42) { [Console]::WindowHeight = [math]::Min(42, [Console]::LargestWindowHeight) } } catch {}
    [Console]::CursorVisible = $false
    try { $MouseOK = [ConIn]::Enable() } catch { $MouseOK = $false }
    Run-List
} catch {
    try { [ConIn]::Restore() } catch {}
    [Console]::CursorVisible = $true
    Clear-Host
    Write-Host "ОШИБКА: $_" -ForegroundColor Red
    Write-Host $_.ScriptStackTrace -ForegroundColor DarkGray
    Read-Host 'Enter — закрыть'
} finally {
    try { [ConIn]::Restore() } catch {}
    [Console]::CursorVisible = $true
    Clear-Host
}
