# Builds PCTwin's tiny test installer (fixtures/msi): a Windows Installer package that installs
# one registry value for everyone and nothing else, made with Windows' own Windows Installer
# (its WindowsInstaller.Installer automation object), so no third-party tool is involved.
#
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File make-test-msi.ps1 -Out <path.msi>
#
# Installing it (needs an administrator):
#   HKLM\SOFTWARE\PCTwinTest  Installed = "1"
# and Windows lists the product under its ProductCode below; removing it takes the value away.
param([Parameter(Mandatory = $true)][string]$Out)

$ErrorActionPreference = 'Stop'
$ProductCode = '{6F1E0A55-3C2B-4D8E-9C1A-50C7EA5E7E01}'
$UpgradeCode = '{6F1E0A55-3C2B-4D8E-9C1A-50C7EA5E7E02}'
$PackageCode = '{6F1E0A55-3C2B-4D8E-9C1A-50C7EA5E7E03}'
$ComponentId = '{6F1E0A55-3C2B-4D8E-9C1A-50C7EA5E7E04}'

if (Test-Path -LiteralPath $Out) { Remove-Item -LiteralPath $Out -Force }
$installer = New-Object -ComObject WindowsInstaller.Installer
# 3: msiOpenDatabaseModeCreate.
$db = $installer.GetType().InvokeMember('OpenDatabase', 'InvokeMethod', $null, $installer, @($Out, 3))

function Run([string]$sql) {
    $view = $db.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $db, @($sql))
    $view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null) | Out-Null
    $view.GetType().InvokeMember('Close', 'InvokeMethod', $null, $view, $null) | Out-Null
}

Run "CREATE TABLE ``Property`` (``Property`` CHAR(72) NOT NULL, ``Value`` LONGCHAR NOT NULL LOCALIZABLE PRIMARY KEY ``Property``)"
Run "CREATE TABLE ``Directory`` (``Directory`` CHAR(72) NOT NULL, ``Directory_Parent`` CHAR(72), ``DefaultDir`` CHAR(255) NOT NULL LOCALIZABLE PRIMARY KEY ``Directory``)"
Run "CREATE TABLE ``Component`` (``Component`` CHAR(72) NOT NULL, ``ComponentId`` CHAR(38), ``Directory_`` CHAR(72) NOT NULL, ``Attributes`` SHORT NOT NULL, ``Condition`` CHAR(255), ``KeyPath`` CHAR(72) PRIMARY KEY ``Component``)"
Run "CREATE TABLE ``Feature`` (``Feature`` CHAR(38) NOT NULL, ``Feature_Parent`` CHAR(38), ``Title`` CHAR(64) LOCALIZABLE, ``Description`` CHAR(255) LOCALIZABLE, ``Display`` SHORT, ``Level`` SHORT NOT NULL, ``Directory_`` CHAR(72), ``Attributes`` SHORT NOT NULL PRIMARY KEY ``Feature``)"
Run "CREATE TABLE ``FeatureComponents`` (``Feature_`` CHAR(38) NOT NULL, ``Component_`` CHAR(72) NOT NULL PRIMARY KEY ``Feature_``, ``Component_``)"
Run "CREATE TABLE ``Registry`` (``Registry`` CHAR(72) NOT NULL, ``Root`` SHORT NOT NULL, ``Key`` CHAR(255) NOT NULL LOCALIZABLE, ``Name`` CHAR(255) LOCALIZABLE, ``Value`` LONGCHAR LOCALIZABLE, ``Component_`` CHAR(72) NOT NULL PRIMARY KEY ``Registry``)"
# RegisterProduct records where the package came from by reading the Media table, and fails the
# whole install (error 2228, then 1603) when there is none, even with no files to copy.
Run "CREATE TABLE ``Media`` (``DiskId`` SHORT NOT NULL, ``LastSequence`` LONG NOT NULL, ``DiskPrompt`` CHAR(64) LOCALIZABLE, ``Cabinet`` CHAR(255), ``VolumeLabel`` CHAR(32), ``Source`` CHAR(72) PRIMARY KEY ``DiskId``)"
Run "CREATE TABLE ``InstallExecuteSequence`` (``Action`` CHAR(72) NOT NULL, ``Condition`` CHAR(255), ``Sequence`` SHORT PRIMARY KEY ``Action``)"

$props = @{
    'ProductCode'     = $ProductCode
    'UpgradeCode'     = $UpgradeCode
    'ProductName'     = 'PCTwin test package'
    'ProductVersion'  = '1.0.0'
    'Manufacturer'    = 'PCTwin maintainers'
    'ProductLanguage' = '1033'
    'ALLUSERS'        = '1'
}
foreach ($k in $props.Keys) { Run "INSERT INTO ``Property`` (``Property``, ``Value``) VALUES ('$k', '$($props[$k])')" }
Run "INSERT INTO ``Directory`` (``Directory``, ``Directory_Parent``, ``DefaultDir``) VALUES ('TARGETDIR', '', 'SourceDir')"
# Attributes 4: the component's key path is a registry value; 256: 64-bit.
Run "INSERT INTO ``Component`` (``Component``, ``ComponentId``, ``Directory_``, ``Attributes``, ``Condition``, ``KeyPath``) VALUES ('Main', '$ComponentId', 'TARGETDIR', 260, '', 'Mark')"
Run "INSERT INTO ``Feature`` (``Feature``, ``Feature_Parent``, ``Title``, ``Description``, ``Display``, ``Level``, ``Directory_``, ``Attributes``) VALUES ('All', '', 'All', '', 0, 1, 'TARGETDIR', 0)"
Run "INSERT INTO ``FeatureComponents`` (``Feature_``, ``Component_``) VALUES ('All', 'Main')"
# One disk, the package itself, holding no files.
Run "INSERT INTO ``Media`` (``DiskId``, ``LastSequence``) VALUES (1, 0)"
# Root 2: HKEY_LOCAL_MACHINE.
Run "INSERT INTO ``Registry`` (``Registry``, ``Root``, ``Key``, ``Name``, ``Value``, ``Component_``) VALUES ('Mark', 2, 'SOFTWARE\PCTwinTest', 'Installed', '1', 'Main')"
$seq = @(
    @('CostInitialize', 800), @('FileCost', 900), @('CostFinalize', 1000), @('InstallValidate', 1400),
    @('InstallInitialize', 1500), @('ProcessComponents', 1600), @('UnpublishFeatures', 1800),
    @('RemoveRegistryValues', 2600), @('WriteRegistryValues', 5000), @('RegisterUser', 6000),
    @('RegisterProduct', 6100), @('PublishFeatures', 6300), @('PublishProduct', 6400),
    @('InstallFinalize', 6600)
)
foreach ($s in $seq) { Run "INSERT INTO ``InstallExecuteSequence`` (``Action``, ``Condition``, ``Sequence``) VALUES ('$($s[0])', '', $($s[1]))" }

# The summary information: 2 title, 3 subject, 4 author, 7 template (x64, English), 9 package
# code, 14 schema (200: 64-bit), 15 word count (2: compressed, none here), 12/13 times.
$info = $db.GetType().InvokeMember('SummaryInformation', 'GetProperty', $null, $db, @(20))
$set = { param($id, $value) $info.GetType().InvokeMember('Property', 'SetProperty', $null, $info, @($id, $value)) | Out-Null }
& $set 2 'Installation Database'
& $set 3 'PCTwin test package'
& $set 4 'PCTwin maintainers'
& $set 7 'x64;1033'
& $set 9 $PackageCode
& $set 14 200
& $set 15 2
$info.GetType().InvokeMember('Persist', 'InvokeMethod', $null, $info, $null) | Out-Null
$db.GetType().InvokeMember('Commit', 'InvokeMethod', $null, $db, $null) | Out-Null
[System.Runtime.InteropServices.Marshal]::ReleaseComObject($db) | Out-Null
Write-Output $ProductCode
