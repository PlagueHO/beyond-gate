#Requires -Version 7.4
[CmdletBinding()]
param([switch]$Check)
$ErrorActionPreference = 'Stop'
$version = '6.2.0'
if (Get-Module -ListAvailable Pester | Where-Object Version -EQ $version) { return }
if ($Check) { throw 'Pester 6.2.0 is required. Run scripts\install-test-tools.ps1.' }
Install-PSResource -Name Pester -Version $version -Repository PSGallery -Scope CurrentUser -TrustRepository
if (!(Get-Module -ListAvailable Pester | Where-Object Version -EQ $version)) {
    throw 'Pester 6.2.0 installation failed.'
}
