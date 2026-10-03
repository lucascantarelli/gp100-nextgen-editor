# Empacotamento e distribuição (#27 · #28 · #29)

Fonte de verdade do "como o app vira arquivo de instalação" e, mais útil, do
**por que cada coisa é do jeito que é**. O que roda é o job `Distribuição · instalador`
do [`ci.yml`](../.github/workflows/ci.yml); o que ele produz é decidedor aqui.

## O mapa: um build, três formatos

```
                       cargo build --release  (mesmo binário)
                                 │
        ┌────────────────────────┼────────────────────────┐
        ▼                        ▼                        ▼
   nsis + msi                dmg (app)              deb + AppImage
   (windows-latest)      (macos-15-intel)          (ubuntu-24.04)
        │                        │                        │
        └────────────┬───────────┴────────────┬───────────┘
                     ▼                        ▼
              Releases do GitHub       PKGBUILD versionado
                                        (Arch — receita, não artefato)
```

O ponto que costuma dar errado: **o PKGBUILD não é um quarto build**. Ele
compila o mesmo tarball da release. Se ele passasse a compilar do branch,
Arch e Debian parariam de servir o mesmo binário e o primeiro bug de
"funciona no meu Arch" seria impossível de comparar.

## `tauri.conf.json` — o que cada campo decide

| Campo | Valor | Por quê |
|---|---|---|
| `mainBinaryName` | `gp100-nextgen-editor` | Sem isso o binário sai como `gp100-ui` (nome do crate). O `.deb` e o `/usr/bin/` do Arch ficariam com nomes diferentes do que o menu de aplicativos executa. |
| `targets` | `nsis, msi, dmg, deb, appimage` | Cobre `Distribuição · instalador` nas três plataformas. O job passa `--bundles`, que **sobrescreve** esta lista — ela existe para o build local sem flag, e o gate `check_bundle.py` compara as duas. |
| `icon` | 5 arquivos | O Tauri v2 exige `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns` (macOS) e `icon.ico` (Windows). |
| `linux.deb.depends` | `libwebkit2gtk-4.1-0`, `libgtk-3-0` | Declarar `depends` **substitui** o default do Tauri. A lista é curta de propósito: o app não usa tray (`capabilities/default.json` só pede `core:default`), então `libayatana-appindicator3-1` seria dependência morta. |
| `windows.wix.language` | `pt-BR`, `en-US` | O público do projeto é pt-BR. |
| `macOS.minimumSystemVersion` | `10.15` | Base do que o WebKit do Tauri 2 suporta. |

## Ícones: derivados, não desenhados duas vezes

O set não é um conjunto de arquivos soltos. A **fonte** é
[`scripts/make_icon.py`](../scripts/make_icon.py), que desenha 1024×1024 a
partir dos tokens do próprio tema (`--surface-top`, `--accent`, `--silver` de
[`design.css`](../packages/app/ui/src/design/design.css)) — pedal visto de
cima: chassi escuro, aro cromado do footswitch, núcleo violeta aceso. Um pedal
lê a 32×32; as letras "GP" ou o número "100" somem antes.

```bash
python3 scripts/make_icon.py                                  # a fonte 1024²
cd packages/app/api && ../../ui/node_modules/.bin/tauri icon icons/icon-source.png -o icons
rm -rf icons/android icons/ios                                # o Tauri também gera mobile
```

Por que um script e não um PNG no repositório: o set é **derivado**, e guardar
só o derivado perde a procedência. Alguém precisa poder responder "de onde veio
essa cor" sem abrir o histórico de arte.

O desenho antigo era um `icon.png` 32×32 de 155 bytes e um `.ico` com **uma**
entrada. O `.ico` fino não quebra o build — produz instalador serrilhado, que
é bem mais difícil de diagnosticar do que um erro. Por isso `check_bundle.py`
confere as resoluções, e não só a existência do arquivo.

## Deb: as duas vias

```bash
sudo apt install ./gp100-nextgen-editor_0.1.0_amd64.deb   # resolve dependências
sudo dpkg -i ./gp100-nextgen-editor_0.1.0_amd64.deb         # NÃO: instala e não abre
```

O README insiste nisso porque é o erro mais comum com `.deb` de Tauri: o
`dpkg` não resolve dependências, e o app só falha ao abrir, sem mensagem
útil.

## Arch: o pacote é a receita

O Tauri não empacota para Arch — o **PKGBUILD** é a unidade de distribuição,
então ele é versionado em [`packaging/arch/`](../packaging/arch):

- `PKGBUILD` — compila o tarball da release com `cargo build --release --locked`;
- `gp100-nextgen-editor.desktop` — entrada XDG, `Exec=gp100-nextgen-editor`.

Duas travas em [`scripts/check_bundle.py`](../scripts/check_bundle.py):

1. `pkgname` do PKGBUILD tem que ser **igual** a `mainBinaryName` da config —
   senão `/usr/bin/<pkgname>` aponta para um binário que não existe;
2. o `Exec=` do `.desktop` tem que casar com os dois.

Pendência conhecida: `sha256sums=('SKIP')` porque a release é um tag gerado em
runtime. Para publicar no AUR é preciso fixar o digest real (`updpkgsums`); o
AUR rejeita `SKIP`.

## O gate

[`scripts/check_bundle.py`](../scripts/check_bundle.py) roda no job `Lint · contratos do pipeline`
e pega, com arquivo:linha:

- campo inexistente na `tauri.conf.json` — validado contra o **schema oficial**
  do Tauri (rede; sem rede vira aviso, porque gate que reprova por falta de
  internet treina a gente a ignorar o vermelho);
- `bundle.icon` apontando para arquivo que não existe;
- `.ico` sem 16/32/48/256;
- `targets` sem o bundle que o job de dist pede;
- `.deb` sem `libwebkit2gtk-4.1-0`;
- PKGBUILD/`.desktop`/`mainBinaryName` divergentes;
- caminho de origem citado no `package()` que não existe.

## Pendências

- **Assinatura/notariação** (#27): `.msi` sem assinatura Authenticode e `.dmg`
  sem notarização. Depende de certificado; o README já documenta o contorno
  (`xattr -dr com.apple.quarantine`).
- **`LICENSE`**: o `Cargo.toml` declara MIT, mas o repositório não tem o arquivo
  de licença. O PKGBUILD declara `license=('MIT')` espelhando o `Cargo.toml` —
  antes de publicar no AUR vale materializar o texto (a escolha é do owner:
  copyright, ano).
