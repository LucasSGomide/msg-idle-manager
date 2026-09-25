# Idle Manager

[English](README.md) | Português (Brasil)

> O `README.md` em inglês é a versão de referência; esta tradução pode ficar
> um passo atrás dele.

O Idle Manager mantém vários jogos idle de navegador rodando ao mesmo tempo em
uma só janela, no Linux e no Windows. Ele é para quem joga alguns desses jogos
com mais de uma conta em cada um e cansou de perdê-los no meio das abas do
navegador: cada conta ganha o seu próprio login isolado, um lugar em uma grade
e uma linha em uma barra lateral, e os jogos continuam rodando enquanto a
janela fica minimizada.

<p align="center">
  <img src="docs/screenshots/main-window-two-games-and-a-parked-account.png" width="800" alt="A janela principal: a barra lateral com três workspaces, dois jogos rodando lado a lado, uma conta estacionada com o botão Start e o rodapé de memória">
</p>
<p align="center"><em>A janela principal: três <code>workspaces</code> na barra lateral, dois jogos lado a lado, uma conta estacionada esperando pelo botão <strong>Start</strong> e o rodapé de memória.</em></p>

## O que ele faz

Alguns termos da interface ainda não têm tradução fechada; eles aparecem em
`código`, com o nome que o app usa hoje: `workspace`, `preset` e `keep-awake`.

- **Contas isoladas em uma só janela.** Cada conta tem os seus próprios cookies
  e o seu próprio armazenamento, então duas contas do mesmo jogo ficam logadas
  lado a lado sem que uma derrube a outra. A janela mostra um, dois ou quatro
  jogos por vez; os demais continuam rodando fora de vista, e `F5` recarrega o
  que está em foco.
- **Uma barra lateral que conhece todas as contas.** Cada conta é uma linha
  com uma marca colorida: verde quando está na tela, âmbar quando roda em
  segundo plano, e uma marca cinza quando está estacionada. Clique em uma linha
  e aquele jogo ocupa o lugar em foco na grade.
- **Estacionar.** Estacione uma conta e ela devolve a memória que usava, mas o
  login continua no disco. **Start** traz a conta de volta em poucos segundos,
  ainda logada.
- **`Keep-awake` para jogos escondidos.** Navegadores desaceleram páginas que
  ninguém está olhando. Marque uma conta para continuar rodando quando oculta
  e ela roda em velocidade total com a janela minimizada, que é o sentido de um
  jogo idle.
- **Memória que dá para ver.** O rodapé mostra quanto o app em si custa,
  quanto as contas rodando custam e o total, então dá para ver quanto
  estacionar uma conta devolve de fato.
- **`Presets`.** Adicionar uma conta é escolher um jogo em uma lista e digitar
  um nome. O endereço, o zoom e a identidade de navegador vêm de um pequeno
  arquivo de texto por jogo: três acompanham o app, os seus ficam na pasta
  `presets/` da configuração, e "Something else…" aceita qualquer endereço.
- **O `workspace` volta.** Cada conta, nome, arranjo e configuração é salvo
  conforme você muda e restaurado uma conta por vez ao abrir, então você pode
  fechar o app quando quiser.
- **`Workspaces`, páginas e zoom.** Agrupe contas em `workspaces` com nome,
  vire páginas em um `workspace` com mais contas do que a grade mostra, e
  redimensione um jogo no lugar com `Ctrl` `+` / `-` / `0` ou `Ctrl` + roda do
  mouse; o zoom é lembrado por arranjo.
- **Teclado em tudo.** `Ctrl` + `?` abre a janela de atalhos abaixo: estacionar
  ou iniciar uma conta ou um `workspace` inteiro, trocar de `workspace`, virar
  uma página, mudar o layout, tudo sem o mouse.

<p align="center">
  <img src="docs/screenshots/shortcuts-dialog.png" width="800" alt="A janela de atalhos sobre a janela principal, listando as combinações para jogos, contas e a janela">
</p>
<p align="center"><em>A janela de atalhos (<code>Ctrl</code> + <code>?</code>): todas as combinações para jogos, contas e a janela.</em></p>

- **Operar uma conta pelo celular.** Abra a janela Phone, leia o código QR, e o
  jogo em foco preenche a tela do celular, transmitido ao vivo do computador e
  diagramado pelo próprio jogo para uma tela de celular. Um toque no celular
  chega como um clique no jogo. É preciso uma rede mesh como o Tailscale, que
  dá ao computador e ao celular endereços em `100.64.0.0/10`; nada é aberto no
  roteador de casa.
- **Versões e atualizações de dentro do app.** Cada versão publica um zip para
  Windows e um AppImage para Linux, construídos do mesmo código-fonte. Uma
  cópia em execução percebe uma versão mais nova, baixa em segundo plano
  enquanto todos os jogos continuam rodando, e instala quando você fecha o
  app, nunca por conta própria.

<p align="center">
  <img src="docs/screenshots/phone-dialog-with-qr-code.png" width="800" alt="A janela Phone com o seu código QR, sobre um único jogo no layout de celular, página 1 de 3">
</p>
<p align="center"><em>A janela Phone: leia o código QR com o celular e o jogo em foco preenche a tela dele. Atrás, a janela está no layout de celular, na página 1 de 3. O código e o endereço estão borrados nesta imagem.</em></p>

## Download

Cada versão publica um arquivo por sistema, os dois construídos do mesmo
código-fonte e com o mesmo número de versão:

| Sistema | Arquivo | Precisa de |
| --- | --- | --- |
| Windows 10 22H2 / 11, 64 bits | `IdleManager-win-Portable.zip` | nada além do que já vem no Windows — o runtime do WebView2 |
| Linux, Ubuntu 24.04+ ou Debian 13+, 64 bits | `IdleManager.AppImage` | o GTK 4 (4.10+) e o WebKitGTK 6.0 (2.42+) do próprio sistema, que essas distribuições já trazem |

Baixe a versão mais recente na
[página de Releases](https://github.com/LucasSGomide/msg-idle-manager/releases/latest).
O `SHA256SUMS` e um `.minisig` ao lado de cada arquivo deixam você conferir à
mão o que baixou; `release/minisign.pub` é a chave pública
([`release/README.md`](release/README.md) explica como esse par funciona).

**Windows.** Descompacte o `IdleManager-win-Portable.zip` em qualquer lugar —
sem instalador, sem direitos de administrador — e execute o `IdleManager.exe`
dentro da pasta `current\` descompactada. Como o programa não é assinado com
um certificado de assinatura de código, a primeira execução mostra o aviso do
próprio Windows, **"O Windows protegeu o computador"** (SmartScreen). Isso é
esperado: clique em **Mais informações** e depois em **Executar assim mesmo**.
Uma máquina com Windows 11 e o Controle Inteligente de Aplicativos ligado pode
recusar o programa de vez em vez de mostrar esse aviso; desligar o Controle
Inteligente de Aplicativos é hoje a única saída.

**Linux.** Baixe o `IdleManager.AppImage`, marque-o como executável
(`chmod +x IdleManager.AppImage`) e execute — sem instalação, sem root, e nada
além do GTK 4 e do WebKitGTK precisa já estar no sistema.

**Atualizando.** O app consulta o GitHub por uma versão mais nova uma vez ao
abrir e uma vez a cada 24 horas, e `Check for updates` no menu `☰` da barra de
título faz a mesma consulta à mão a qualquer momento. Quando existe uma versão
mais nova, um aviso oferece **Update**: o download acontece em segundo plano
enquanto todos os jogos continuam rodando, a assinatura do arquivo é
verificada, e então o aviso oferece **Restart now**. A atualização é instalada
no momento em que você sai, de qualquer jeito — por `Restart now` ou fechando o
app de qualquer outra forma — e cada conta, login, `workspace`, `preset` e
nível de zoom está exatamente onde você deixou.

## Documentação

- [`docs/stack.md`](docs/stack.md) — com o que o app é construído, em qual
  versão, e por quê.
- [`docs/architecture.md`](docs/architecture.md) — os crates, a única regra de
  dependência, e onde uma mudança entra.
- [`docs/code-standards.md`](docs/code-standards.md) — como o código dentro
  de um crate é escrito.
- [`docs/naming.md`](docs/naming.md) — como arquivos, crates e identificadores
  são nomeados.
- [`docs/design.md`](docs/design.md) — as regras numeradas que toda tela segue.
- [`docs/requirements.md`](docs/requirements.md) — o registro, só de acréscimos,
  de necessidades do usuário e requisitos funcionais.
- [`docs/roadmap/README.md`](docs/roadmap/README.md) — todos os itens do
  roadmap, o que vem a seguir, e por quê.
- [`CHANGELOG.md`](CHANGELOG.md) — o que cada versão mudou.
- [`release/README.md`](release/README.md) — a chave que assina as versões e
  como conferir um download.

## Licença

Copyright © 2026 Lucas Gomide. O Idle Manager é software livre sob a
[GNU Affero General Public License, versão 3](LICENSE): você pode executar,
estudar, compartilhar e modificar o programa, e quem distribui uma versão
modificada, ou a executa para outras pessoas pela rede, precisa publicar o
código-fonte completo sob a mesma licença.
