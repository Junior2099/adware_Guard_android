# AdwareGuard — Windows / Rust

Interface em português para inspecionar aplicativos Android por USB usando ADB. Projeto-fonte com eframe/egui 0.31.1. Não é antivírus e não faz análise do conteúdo ou assinatura dos APKs.

## 1. Instalar Rust

Baixe o rustup em https://www.rust-lang.org/tools/install e instale o Rust estável para Windows (MSVC). Se solicitado, instale Visual Studio Build Tools com a carga de trabalho **Desenvolvimento para desktop com C++** e Windows SDK. Reinicie o terminal e confira:

```bat
rustc --version
cargo --version
```

## 2. Baixar o ADB

Baixe **SDK Platform-Tools para Windows** em https://developer.android.com/tools/releases/platform-tools . Extraia o conteúdo da pasta platform-tools para `tools/platform-tools/` neste projeto. Mantenha juntos `adb.exe`, `AdbWinApi.dll`, `AdbWinUsbApi.dll` e os demais arquivos oficiais. Nenhum binário ADB acompanha este ZIP.

```text
AdwareGuard/
  Cargo.toml
  src/main.rs
  src/core.rs
  data/confirmed_malware.txt
  tools/platform-tools/adb.exe
  tools/platform-tools/AdbWinApi.dll
  tools/platform-tools/AdbWinUsbApi.dll
  compilar-windows.bat
  README.md
```

## 3. Preparar o celular

Em Configurações → Sobre o telefone, toque sete vezes em Número da versão (pode estar em Informações do software). Ative **Depuração USB** em Opções do desenvolvedor. Conecte um cabo com transmissão de dados, desbloqueie o celular e aceite a autorização RSA deste computador. Use apenas aparelhos seus ou que você tem autorização para administrar.

## 4. Testar o ADB

No Prompt de Comando, na pasta do projeto:

```bat
tools\platform-tools\adb.exe devices
```

`device` significa autorizado; `unauthorized` exige aceitar a autorização na tela do aparelho; `offline` pode exigir reconectar. Alguns fabricantes exigem driver USB oficial no Windows. O programa seleciona um serial explicitamente em cada comando, permitindo escolher entre vários celulares.

## 5. Conferir o projeto

Abra o terminal na pasta de `Cargo.toml`. A pasta `data` deve estar presente. A lista `confirmed_malware.txt` começa **vazia intencionalmente**, para não atribuir malware a pacotes sem evidência.

## 6. Compilar e executar

Requer internet para obter as dependências na primeira compilação.

```bat
cargo check
cargo test
cargo run
```

Versão de distribuição:

```bat
cargo build --release
```

O executável será `target/release/adwareguard.exe`. Para montar uma pasta portátil com executável, dados e ADB, execute `compilar-windows.bat`. Abra `dist/adwareguard.exe` e mantenha as pastas `data` e `tools` juntas dele. Também é possível informar o caminho completo de outro `adb.exe` no campo ADB da interface.

O caminho padrão usa a pasta do executável quando ela contém `tools/platform-tools`; durante `cargo run`, usa a pasta do projeto. Não mova apenas o EXE. O arquivo `.github/workflows/windows.yml` permite compilar e baixar um artefato pelo GitHub Actions ao colocar o conteúdo desta pasta na raiz de um repositório; o ADB continua precisando ser adicionado separadamente.

## 7. Analisar aplicativos e remover individualmente

1. Clique em **Detectar dispositivos USB** e selecione um aparelho autorizado.
2. Clique em **Analisar aplicativos / atualizar**.
3. O programa consulta o usuário Android em primeiro plano e lista seus pacotes de terceiros e do sistema. Exibe identificadores de pacote e permite carregar o nome e o ícone originais sob demanda com o AAPT2 opcional.
4. Revise o pacote, sua origem e a razão do alerta. Apps do sistema ficam protegidos, sem opção de remoção.
5. Para remover um app de terceiros, clique em **Revisar remoção**, leia o aviso, marque a caixa de confirmação e clique em Confirmar e remover. A remoção pode apagar dados do app: faça backup antes.
6. ADB precisa retornar `Success` e o pacote deve deixar de aparecer na listagem do usuário analisado para que o contador aumente. Falhas ou verificações incompletas não são contadas. Em caso de timeout, a operação pode já ter ocorrido: atualize a análise antes de repetir.
7. Clique em **Finalizar e mostrar resumo** para ver o resultado; ele também fica sempre visível após análise e remoções.

A remoção usa `pm uninstall --user ID PACOTE`, para o usuário analisado. Outros usuários/perfis podem continuar com o app. Uma troca de usuário Android é detectada antes da remoção e exige nova análise. Não há root, remoção em massa ou desinstalação automática.

### Classificação e resumo

- **LISTA LOCAL**: correspondência exata de pacote com uma entrada mantida por você, com fonte HTTPS. A lista é editável e a fonte não é validada pelo programa; o nome sozinho não prova que o APK instalado é o mesmo do relatório.
- **Suspeita por nome**: termos `adware`, `popup`, `pushads` ou `clicker` no identificador. É um sinal fraco, com falsos positivos; muitos adwares não contêm esses termos.
- **Sem indício pelas regras**: não há evidência disponível neste método; não significa que o aplicativo seja seguro.

O resumo mostra **Apps da lista de nocivos eliminados**, **Outros apps removidos**, **Suspeitas por nome** e correspondências ainda instaladas. Os contadores só representam remoções verificadas pelo programa durante esta sessão para o aparelho selecionado. Detectar de novo ou trocar o aparelho reinicia os contadores. Atualizar a análise no mesmo aparelho mantém os contadores.

Quando nenhuma correspondência tiver sido encontrada ou removida, mostra: **“Não foi encontrado nenhum app nocivo pelas regras disponíveis. Isso não garante ausência de malware.”** Alertas heurísticos continuam separados. Erro de leitura não é apresentado como análise limpa. O histórico fica em memória e é perdido ao fechar o programa; nada é enviado a um serviço de análise.

### Manter a lista local

Edite `data/confirmed_malware.txt`, com uma entrada por linha:

```text
# nome.exato.do.pacote | https://fonte-confiavel/relatorio
```

O exemplo está comentado para não ativar regra fictícia. Inclua somente pacotes com relatório confiável e confira variante, assinatura e data por meios externos. Linhas sem fonte HTTPS ou pacotes inválidos interrompem a análise com erro. Não há atualização automática da lista, verificação da fonte ou detecção por assinatura. Reanalise depois de editar.

## 8. Configurar DNS privado pelo Android

Em Android 9 ou superior: Configurações → Rede e Internet ou Conexões → DNS privado → Nome do host do provedor de DNS privado:

```text
dns.adguard-dns.com
```

Salve e teste a navegação. O caminho pode variar. O provedor receberá as consultas DNS direcionadas a ele. O DNS pode filtrar muitos domínios de publicidade e rastreamento, mas não remove apps, não bloqueia todos os anúncios e não substitui análise de segurança.

## 9. Configurar por ADB (opcional)

O botão **Configurar DNS AdGuard** pede confirmação e tenta gravar:

```bat
adb -s SERIAL shell settings put global private_dns_specifier dns.adguard-dns.com
adb -s SERIAL shell settings put global private_dns_mode hostname
```

O programa lê de volta ambos os valores. Isso confirma os valores gravados, **não a conectividade nem o funcionamento do bloqueio**. Confira no menu do Android e teste a rede. Fabricantes podem restringir os comandos. A configuração global pode afetar todos os usuários do celular.

Em falhas, o programa tenta restaurar os valores anteriores e informa o resultado da tentativa; confirme sempre no Android. **DNS automático** grava `private_dns_mode=opportunistic`, equivalente à opção Automático. O hostname salvo pode permanecer, mas não é usado nesse modo. Também é possível voltar manualmente a Automático nas configurações do telefone.

## 10. Limites e segurança

Não existe garantia de identificar todos os adwares por identificador de pacote. O app não analisa permissões, certificados, comportamento, arquivos APK ou tráfego, e não identifica apps nocivos de forma definitiva. A lista inicial vazia significa que nenhum pacote será contabilizado como nocivo por padrão. O programa não inclui regras fictícias para produzir resultados.

As ações ADB rodam em uma thread separada; a interface fica disponível e impede ações concorrentes. Cada comando tem limite de 45 segundos. Use o computador e o ADB de uma fonte confiável. Ao terminar, você pode desativar a Depuração USB ou revogar autorizações USB no Android. Projeto independente, sem afiliação com AdGuard, Google ou Rust.

## 11. Resolução de problemas

- **Celular não aparece:** confira cabo de dados, porta, driver do fabricante, depuração USB e autorização. Se necessário, selecione transferência de arquivos no celular.
- **ADB não inicia:** confira o caminho do EXE e as DLLs. Teste `adb devices` no terminal.
- **Falha no cargo check:** leia a primeira mensagem, atualize Rust com `rustup update stable` e confira Build Tools C++, Windows SDK e internet.
- **Lista indisponível/inválida:** restaure `data/confirmed_malware.txt` e confira o formato das entradas.
- **Remoção recusada:** políticas de administrador, perfil de trabalho ou proteção do fabricante podem impedir a operação. O programa não tenta contornar essas restrições.
- **DNS sem internet:** selecione Automático no celular. Algumas redes bloqueiam DNS sobre TLS ou exigem portal de autenticação.

## Validação desta entrega

Nesta versão 0.2.0, `cargo check` e `cargo check --tests` passaram no ambiente Linux. Também passou `cargo check --target x86_64-pc-windows-msvc`, verificando o código para Windows sem gerar o executável. Os três testes de validação de pacotes/lista foram compilados e executados diretamente com `rustc --test src/core.rs`: todos passaram. O programa não foi executado graficamente neste ambiente sem desktop nem testado com um Android conectado. O ZIP contém código-fonte, não um EXE compilado. A etapa 6 compila e testa no Windows.

## Referências oficiais

- Rust: https://www.rust-lang.org/tools/install
- eframe 0.31.1: https://docs.rs/eframe/0.31.1/eframe/
- Platform-Tools: https://developer.android.com/tools/releases/platform-tools
- ADB: https://developer.android.com/tools/adb
- Usuários Android/ADB: https://source.android.com/docs/devices/admin/multi-user-testing
- DNS privado Android: https://support.google.com/android/answer/9654714
- DNS público AdGuard: https://adguard-dns.io/en/public-dns.html

## Correção 0.1.1

Corrigida a validação de `package:android`: o framework usa esse nome sem ponto. O pacote é listado como sistema e tem proteção explícita contra remoção. Os demais identificadores continuam sujeitos à validação e respostas inesperadas continuam produzindo erro. Teste de regressão incluído e aprovado na validação da versão 0.2.0.

Para atualizar sem perder o ADB ou sua lista local: feche o aplicativo, substitua `src/core.rs` e `Cargo.toml` na pasta do projeto pelos desta versão, preserve `tools` e `data` e execute `compilar-windows.bat`. Abra o novo `dist/adwareguard.exe`. Não substitua sua pasta `data` pela lista vazia do ZIP.

## Interface 0.2.0

Interface redesenhada em azul escuro e verde, com conexão e DNS na lateral, cartões de resultados, busca por nome/pacote, filtros (Todos, Para revisar, Terceiros, Sistema e Removidos), histórico recolhível e confirmação centralizada. A janela e a barra de tarefas recebem o ícone do AdwareGuard.

### Confirmação por caixa de seleção

Clique em Revisar remoção. O diálogo mostra nome, pacote, aparelho e usuário Android. Marque **Entendo e desejo remover este aplicativo** para liberar **Confirmar e remover**. A caixa começa desmarcada a cada abertura, inclusive após cancelar. Todas as verificações de remoção da versão anterior foram mantidas: bloqueio do sistema, usuário, instalação e confirmação de sucesso/ausência. O nome do pacote não precisa mais ser digitado.

### Nomes e ícones dos apps (opcional)

O ADB não fornece o ícone original em `pm list packages`. Para obter o recurso correto, instale Android SDK Build-Tools pelo SDK Manager do Android Studio e localize `aapt2.exe`, normalmente em `%LOCALAPPDATA%/Android/Sdk/build-tools/VERSAO/aapt2.exe`. Use uma versão oficial recente, com seus arquivos acompanhantes. Não baixe esse executável de fontes desconhecidas.

Em **Caminhos avançados**, informe o caminho completo para `aapt2.exe`, ou coloque-o em `tools/aapt2/aapt2.exe` no projeto e recompile/empacote. Na lista, clique em **Carregar nome e ícone** no app desejado. O programa copia o APK instalado para uma pasta temporária, lê o nome e o caminho do ícone com AAPT2 e exibe imagens PNG, JPEG ou WebP do APK. A pasta temporária é apagada ao final da operação; nada é enviado a um serviço externo nem instalado no celular. O nome exibido não altera a identificação por pacote ou as regras de análise.

Não é preciso configurar AAPT2 para analisar, remover ou mudar DNS. Sem ele, aparece um avatar com inicial, identificado como alternativa ao passar o mouse. Ícones adaptativos em XML, vetoriais ou recursos em APKs divididos podem não ser exibidos; nesses casos o avatar é mantido e o motivo é informado. A leitura é por app, sob demanda, para não copiar centenas de APKs durante a análise. O resultado fica em memória para o dispositivo selecionado e é descartado ao redetectar, trocar de aparelho ou fechar o programa.

### Atualizar preservando seus arquivos

Feche o app. No projeto antigo, substitua **Cargo.toml** e a pasta **src** pelos desta versão (inclui o novo `icons.rs`). Preserve sua pasta `tools`, o ADB e sua lista em `data`. Execute `compilar-windows.bat` e abra o novo `dist/adwareguard.exe`. Se houver um Cargo.lock antigo, o Cargo o atualizará conforme necessário; a nova compilação baixa dependências para leitura de imagens e APKs. Nenhum binário ADB/AAPT2 acompanha o ZIP.

## Versão 0.3.0 — temas e categorias de análise

O interruptor no cabeçalho alterna entre **Tema claro** e **Tema escuro**, incluindo painéis, lista, textos, campos, botões e diálogos de confirmação. A mudança não interrompe a análise nem altera o aparelho selecionado; a preferência vale para a sessão atual e o app inicia no tema escuro.

A lista agora tem filtros com contadores e seções separadas:

- **Lista local:** correspondência exata na lista editável, exige revisão da fonte e da identidade do APK. Não significa infecção comprovada.
- **Suspeitas por nome:** heurística fraca, sujeita a falsos positivos.
- **Sem indícios:** nenhum sinal encontrado pelas regras disponíveis; não é uma garantia de segurança.
- **Removidos:** pacotes com remoção verificada na análise atual. O contador de remoções da sessão permanece no painel.

O filtro **Todos os tipos / Terceiros / Sistema** funciona junto da busca e da categoria escolhida. Um pacote do sistema pode aparecer em qualquer categoria de análise, mantendo sua proteção contra remoção. Em Todos, os grupos aparecem nesta ordem: lista local, suspeitas por nome, sem indícios e removidos. Os contadores das abas mostram o total geral da análise; os cabeçalhos das seções mostram o resultado após busca e filtro de tipo. Cada pacote aparece em uma única categoria, com prioridade de removido, correspondência na lista, suspeita por nome e sem indícios.

Para atualizar, substitua `Cargo.toml`, `Cargo.lock` e a pasta `src` no projeto antigo pelos desta versão, preserve `tools` e `data` e execute `compilar-windows.bat`. A confirmação individual por caixa de seleção e todas as verificações de remoção foram mantidas.

Validação da versão 0.3.0: `cargo check --target x86_64-pc-windows-msvc` aprovado e `cargo test` aprovado (4 testes, incluindo separação das categorias sem ocultar alertas em apps do sistema). A execução visual no Windows e em um aparelho conectado continua dependendo da conferência no computador do usuário.

## Versão 0.3.1 — termos de revisão ampliados

A heurística por nome agora considera `clear`, `clean` (também alcança cleaner), `PDF`, `PDF viewer`, `visualizador PDF`, `laucher`, `launcher`, `off-line`, `offline`, `song`, `baixar vídeos`, `videosgratis`, `lanterna` e os termos anteriores `adware`, `popup`, `pushads` e `clicker`.

Também foram incluídos `flashlight`, `QR scanner`, `QR code scanner`, `leitor QR`, `QR code`, `barcode` e `código de barras`. Há campanhas históricas documentadas usando apps de lanterna/limpeza e leitores de código de barras; isso não permite concluir que apps dessas categorias sejam geralmente maliciosos:

- Check Point Research, LightsOut (2018): https://research.checkpoint.com/2018/malicious-flashlight-apps-google-play/
- Trend Micro, leitores de código de barras e adware (2020): https://www.trendmicro.com/pt_br/about/newsroom/press-releases/2020/2020-07-07.html

A comparação é por trecho do texto, sem diferenciar maiúsculas, acentos, espaços, pontuação ou hífens. É deliberadamente abrangente e pode gerar muitos falsos positivos, inclusive em apps legítimos conhecidos. O alerta apresenta os termos encontrados e indica **pacote**, **nome exibido** ou **pacote e nome exibido**. Não é um diagnóstico, não adiciona o aplicativo à lista de malware e não incrementa o contador de nocivos eliminados.

A análise inicial usa o identificador do pacote fornecido pelo ADB. O nome comercial é analisado quando você clica em **Carregar nome e ícone** e o AAPT2 consegue obtê-lo; a categoria e os contadores são atualizados nesse momento. Ao reanalisar no mesmo usuário/dispositivo, os nomes já carregados na sessão são considerados novamente. Sem AAPT2, a análise por pacote continua funcionando, mas nomes comerciais desconhecidos não podem ser pesquisados. Nenhuma busca online ou conclusão sobre assinatura do APK é feita.

Para atualizar, substitua `Cargo.toml`, `Cargo.lock` e a pasta `src`; preserve sua lista local e o ADB nas pastas `data` e `tools`. Depois execute `compilar-windows.bat`.

Validação da versão 0.3.1: `cargo test` aprovado (5 testes) e `cargo check --target x86_64-pc-windows-msvc` aprovado. Os testes novos verificam termos solicitados, acentos, maiúsculas, separadores e origem do alerta. Sem nova execução gráfica ou em aparelho Android neste ambiente.
