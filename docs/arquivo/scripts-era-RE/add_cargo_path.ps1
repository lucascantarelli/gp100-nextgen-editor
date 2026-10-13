# add_cargo_path.ps1 - adiciona C:\Users\<user>\.cargo\bin ao PATH DO SISTEMA (HKLM).
#
# Por que isso existe (knowledge.md / ROADMAP P2):
#   O winget instalou o rustup em 28/09/2026, mas terminais novos não herdavam o
#   PATH do cargo: todo shell precisava de
#     export PATH="/c/Users/Canta/.cargo/bin:$PATH"
#   antes de qualquer cargo/clippy. Causa: o PATH do USUARIO (HKCU) tem .cargo\bin,
#   mas o do SISTEMA (HKLM) nao - e alguns ambientes (servicos, IDEs, CI local,
#   terminais de outros usuarios) so enxergam o PATH de maquina.
#
# O que este script faz (idempotente - pode rodar quantas vezes quiser):
#   1. Detecta o .cargo\bin do usuario atual (via $env:USERPROFILE, com fallback
#      para CARGO_HOME se definida).
#   2. Le o Path de MAQUINA (HKLM\...\Session Manager\Environment) preservando o
#      tipo do valor (REG_EXPAND_SZ na maioria dos Windows; o metodo ingenuo do
#      [Environment]::SetEnvironmentVariable degrada para REG_SZ e quebra vars
#      do tipo %SystemRoot% - por isso vamos direto ao registro).
#   3. Se .cargo\bin ainda nao estiver la, APPEND no fim (nunca reordena) e grava
#      de volta como REG_EXPAND_SZ (RegistryValueKind.ExpandString).
#   4. Broadcast WM_SETTINGCHANGE (SendMessageTimeout) para o Explorer propagar
#      o novo PATH sem logoff (terminais JAH ABERTOS continuam com o antigo -
#      abrir terminal novo resolve).
#
# Uso (requer elevacao - PowerShell como Administrador):
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\add_cargo_path.ps1
#
# Rollback: remover manualmente em "Editar variaveis de ambiente do sistema" ou
#   (reg delete "HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment" /v Path /f NAO -
#   isso apagaria o Path inteiro; edite o valor e remova so a entrada .cargo\bin).

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# --- 1. alvo: <perfil>\.cargo\bin do usuario atual ----------------------------
$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
$cargoBin  = Join-Path $cargoHome 'bin'

if (-not (Test-Path $cargoBin)) {
    Write-Error "Diretorio esperado do rustup nao existe: $cargoBin (rustup instalado?)"
}
# Sanidade: o alvo deve terminar em \.cargo\bin (padrao do rustup).
# NOTA de regex: em PowerShell, '\b' e word boundary, NAO barra invertida -
# por isso o padrao usa classe de barras '[\\/]'.
if ($cargoBin -notmatch '[\\/]\.cargo[\\/]bin$') {
    Write-Warning "Alvo fora do padrao do rustup: $cargoBin"
}

# --- 2. leitura do Path de MAQUINA preservando metadados ---------------------
$envKey = 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
$pathValue = Get-ItemProperty -Path $envKey -Name Path
$kind = $pathValue.Path.GetType()   # tipo .NET do valor lido (informativo)
$entries = $pathValue.Path -split ';' | Where-Object { $_ -ne '' }

Write-Host "Path de maquina: $($entries.Count) entradas (valor do tipo $($pathValue.Path.GetType().Name))"

# --- 3. idempotencia: ja esta presente? ---------------------------------------
if ($entries -contains $cargoBin) {
    Write-Host "[OK] $cargoBin ja esta no Path do sistema. Nada a fazer." -ForegroundColor Green
    exit 0
}

# Append no FIM (nunca reordenar o PATH de maquina - risco de sombrear
# System32 etc.; rustup no fim e seguro pois e independente).
$newPath = ($entries + $cargoBin) -join ';'

# Grava no registro com o MESMO kind de antes (ExpandString = REG_EXPAND_SZ).
Set-ItemProperty -Path $envKey -Name Path -Value $newPath -Type ExpandString

# Releitura de confirmacao (prova que gravou).
$check = (Get-ItemProperty -Path $envKey -Name Path).Path -split ';'
if ($check -notcontains $cargoBin) { Write-Error 'Falha: entrada nao persistiu no registro.' }

# --- 4. broadcast WM_SETTINGCHANGE --------------------------------------------
# Sem isso, processos novos lancados pelo Explorer continuam com o PATH antigo
# ate o proximo logoff.
$sig = '[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)] ' +
       'public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, ' +
       'UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);'
$type = Add-Type -MemberDefinition $sig -Name Win32SendMessage -Namespace Win32 -PassThru
$result = [UIntPtr]::Zero
$null = $type::SendMessageTimeout([IntPtr]0xffff, 0x001A, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$result)

Write-Host "[OK] Adicionado ao Path do SISTEMA: $cargoBin" -ForegroundColor Green
Write-Host "     (terminais ja abertos mantem o Path antigo; abra um terminal NOVO)"
