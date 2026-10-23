# AdwareGuard

**Versão 0.7.0 · Windows · Rust · Android por USB**

Aplicativo com interface em português para revisar aplicativos Android, remover pacotes selecionados com confirmação, configurar DNS privado e acessar recursos de manutenção do celular usando ADB.

> **O AdwareGuard não é um antivírus.** Ele não analisa o conteúdo, a assinatura ou o comportamento dos APKs para determinar infecção. Alertas por nome são sinais fracos para revisão, e “Sem indícios” não significa que um app seja seguro.

## Funcionalidades

| Recurso | Como funciona |
| --- | --- |
| Conexão USB | Detecta dispositivos autorizados e permite escolher o aparelho. |
| Análise de apps | Lista os pacotes do usuário Android em primeiro plano. |
| Categorias | Separa lista local, suspeitas por nome, sem indícios e removidos. |
| Busca e filtros | Busca por pacote/nome carregado e filtra apps do sistema ou de terceiros. |
| Remoção individual ou em lote | Exige revisão da lista, caixa de confirmação e clique final. |
| Proteção do sistema | Bloqueia a remoção de pacotes identificados como apps do sistema. |
| Nomes e ícones | Carrega sob demanda com AAPT2 opcional; usa avatar quando não consegue ler o ícone. |
| DNS privado | Configura AdGuard DNS ou volta ao modo automático, com confirmação. |
| Animações | Ajusta as escalas e tenta configurar opções extras de acessibilidade expostas pelo fabricante. |
| Cache dos navegadores | Limpa somente o cache de Chrome, Brave ou Firefox quando o aparelho confirma suporte. |
| Cache do aparelho | Oferece prévia e confirmação para miniaturas regeneráveis em duas pastas específicas. |
| Files do Google | Abre a página oficial na Play Store para instalação confirmada no celular. |
| Play Protect | Abre a Play Store e orienta iniciar a verificação no aparelho. |
| Interface | Tema claro por padrão, opção de tema escuro e histórico da sessão. |

## 1. Requisitos

- Computador Windows com acesso USB ao celular.
- Android com **Depuração USB** ativada e autorização concedida a esse computador.
- Cabo USB com transmissão de dados.
- **ADB oficial**, obtido no pacote Android SDK Platform-Tools.
- Para compilar: Rust estável e ferramentas de compilação C++ do Visual Studio, com Windows SDK.
- Para nomes e ícones: AAPT2 do Android SDK Build-Tools, opcional.

Internet é necessária para baixar dependências na primeira compilação, instalar o Files pela loja e usar os serviços online correspondentes. As funções dependem das permissões e restrições do fabricante; não exigem nem tentam obter root.

## 2. Baixar o ADB — obrigatório

**O ADB não acompanha o projeto. Sem ele, o programa não consegue se comunicar com o celular.**

Baixe **SDK Platform-Tools para Windows** no site oficial:

https://developer.android.com/tools/releases/platform-tools

Extraia o **conteúdo** da pasta `platform-tools` para `tools/platform-tools/`, dentro do projeto. Mantenha o pacote oficial completo, incluindo:

```text
tools/platform-tools/
├── adb.exe
├── AdbWinApi.dll
└── AdbWinUsbApi.dll
```

Evite uma pasta duplicada como `platform-tools/platform-tools/adb.exe`.

Se estiver usando a distribuição em `dist`, o caminho esperado será:

```text
dist/tools/platform-tools/adb.exe
```

Também é possível indicar outro caminho completo para `adb.exe` em **Caminhos avançados** na interface. Não copie apenas o EXE sem as DLLs acompanhantes.

## 3. Preparar o celular

1. Abra **Configurações → Sobre o telefone**. O número da versão pode estar em Informações do software.
2. Toque sete vezes em **Número da versão** para habilitar as Opções do desenvolvedor.
3. Ative **Depuração USB**.
4. Conecte o cabo USB, desbloqueie a tela e aceite a autorização de depuração desse computador.
5. Se necessário, selecione Transferência de arquivos no menu USB do celular.

Use apenas aparelhos seus ou que você tenha autorização para administrar. Alguns modelos exigem driver USB oficial do fabricante no Windows.

Teste pelo Prompt de Comando, na pasta do projeto:

```bat
tools\platform-tools\adb.exe devices
```

| Resultado | O que fazer |
| --- | --- |
| `device` | Dispositivo autorizado; pode continuar. |
| `unauthorized` | Desbloqueie o celular e aceite a autorização USB. |
| `offline` | Reconecte o cabo e tente novamente. |
| Lista vazia | Confira cabo de dados, porta USB, driver e Depuração USB. |

## 4. Compilar e executar

Instale o Rust pelo site oficial:

https://www.rust-lang.org/tools/install

Quando solicitado, instale Visual Studio Build Tools com **Desenvolvimento para desktop com C++** e Windows SDK. Reabra o terminal e confira:

```bat
rustc --version
cargo --version
```

Na pasta que contém `Cargo.toml`, execute:

```bat
cargo check
cargo test
cargo run
```

Para gerar a versão de distribuição:

```bat
cargo build --release
```

O EXE será gerado em `target/release/adwareguard.exe`.

### Montar a pasta portátil

Execute:

```bat
compilar-windows.bat
```

O script verifica, testa, compila e copia o executável, `data`, `tools` e este README para `dist`. Depois abra:

```text
dist/adwareguard.exe
```

Mantenha `data` e `tools` ao lado do executável. O script copia os arquivos que você adicionou em `tools`; ele não baixa ADB ou AAPT2.

O projeto também inclui um workflow em `.github/workflows/windows.yml` para verificar e compilar no GitHub Actions, se colocado na raiz de um repositório. O ADB precisa ser adicionado separadamente à distribuição.

## 5. Analisar aplicativos

1. Clique em **1. Detectar dispositivos**.
2. Selecione o aparelho autorizado.
3. Clique em **2. Analisar aplicativos**.
4. Use a busca, as categorias e o filtro de tipo para revisar os resultados.

A análise inicial usa os identificadores dos pacotes. O nome comercial entra na análise após ser carregado com **Carregar nome e ícone**. Apps do sistema e de terceiros são apresentados separadamente pelo filtro de tipo; isso não substitui a categoria de análise.

### Categorias de análise

| Categoria | Significado |
| --- | --- |
| **Lista local** | O pacote corresponde exatamente a uma entrada da lista mantida pelo usuário. É necessário conferir a fonte e a identidade do APK. |
| **Suspeitas por nome** | O pacote ou nome exibido carregado contém um termo de revisão. Pode haver muitos falsos positivos. |
| **Sem indícios** | Nenhuma regra disponível encontrou um sinal. Não é uma garantia de segurança. |
| **Removidos** | Pacotes cuja remoção foi verificada na análise atual. |

Em **Todos**, os aplicativos são agrupados nessas seções. Os contadores das categorias mostram o total da análise; os cabeçalhos das seções refletem a busca e o filtro de tipo aplicados.

### Termos de revisão

São considerados, entre outros aliases incluídos no código:

- `adware`, `popup`, `pushads`, `clicker`;
- `clear`, `clean` (inclui nomes como cleaner);
- `PDF`, `PDF viewer`, `visualizador PDF`;
- `laucher`, `launcher`;
- `off-line`, `offline`, `song`;
- `baixar vídeos`, `videosgratis`, `lanterna`, `flashlight`;
- termos de leitor QR e código de barras, como `QR scanner`, `QR code`, `leitor QR`, `barcode` e `código de barras`.

A busca por esses termos ignora maiúsculas, acentos, espaços, pontuação e hífens. O alerta informa qual termo foi encontrado e se veio do pacote, do nome exibido ou de ambos.

**Esses nomes também são usados por aplicativos legítimos. Um alerta não comprova adware e não coloca o app na lista local de nocivos.**

### Lista local

O arquivo `data/confirmed_malware.txt` começa vazio intencionalmente. Para adicionar uma entrada, use um identificador exato e uma fonte HTTPS confiável:

```text
# Formato de uma entrada, comentado para não ativar regra fictícia:
# nome.exato.do.pacote | https://fonte-confiavel/relatorio
```

Confira data, variante, assinatura e origem por meios externos antes de incluir um pacote. O programa verifica o formato da linha, mas não valida o conteúdo da fonte nem a assinatura do APK. Nomes de pacote podem ser reutilizados. Linhas inválidas interrompem a análise com erro; a lista não recebe atualizações automáticas.

## 6. Remover aplicativos com confirmação

### Seleção múltipla

1. Marque as caixas dos aplicativos que deseja revisar.
2. Clique em **Revisar e remover selecionados**.
3. Confira a lista completa, o aparelho e o usuário Android.
4. Marque **Entendo e desejo remover todos os aplicativos desta lista**.
5. Clique em **Confirmar e remover**.

As seleções são mantidas ao mudar filtros, inclusive quando alguns itens ficam ocultos. O diálogo final mostra todos os selecionados. **Limpar seleção** desmarca todos. Detectar novamente, trocar o dispositivo ou iniciar outra análise limpa a seleção.

O botão **Revisar remoção** de cada app continua disponível e abre a mesma confirmação com um único item. Cancelar não remove nada, e a caixa de confirmação começa desmarcada ao reabrir o diálogo.

> A desinstalação pode apagar os dados do aplicativo. Faça backup antes e confira a origem de cada pacote.

As remoções ocorrem uma por vez. Antes de cada uma, o programa verifica o usuário Android, a instalação e a proteção de apps do sistema. Só conta um sucesso quando o ADB retorna `Success` e o pacote deixa de aparecer na listagem do usuário analisado.

A remoção usa `pm uninstall --user ID PACOTE`. Outros usuários ou perfis podem continuar com o app. Não há remoção automática por alerta de nome, e aplicativos do sistema ficam protegidos.

### Resumo e histórico

O resumo distingue apps da lista local de nocivos eliminados, outros apps removidos, suspeitas por nome e correspondências ainda instaladas. Se nenhuma correspondência tiver sido encontrada ou removida, informa que não encontrou nocivos pelas regras disponíveis, sem garantir ausência de malware.

No lote, falhas ou resultados sem confirmação não aumentam os contadores; os itens com falha permanecem selecionados para revisão. Após timeout, reanalise antes de repetir: uma operação pode já ter ocorrido.

Os contadores são da sessão para o aparelho selecionado. Reanalisar mantém os contadores de remoção, mas reconectar pela detecção ou trocar o aparelho os reinicia. O histórico fica em memória e é perdido ao fechar o programa.

## 7. Nomes e ícones com AAPT2 — opcional

Instale Android SDK Build-Tools pelo SDK Manager do Android Studio. Localize o `aapt2.exe`, geralmente em:

```text
%LOCALAPPDATA%/Android/Sdk/build-tools/VERSAO/aapt2.exe
```

Informe seu caminho em **Caminhos avançados**, ou coloque os arquivos necessários da ferramenta oficial em `tools/aapt2/` e gere a distribuição novamente.

Clique em **Carregar nome e ícone** no app desejado. O programa copia o APK para uma pasta temporária, consulta o AAPT2 e tenta exibir o nome e o recurso de imagem indicado pelo APK. A pasta temporária é removida ao final da operação.

Ícones adaptativos/XML, vetoriais ou localizados em APKs divididos podem não ser exibidos. Nesses casos, permanece um avatar identificado como alternativa. A leitura é sob demanda e o resultado fica em memória na sessão. AAPT2 não é necessário para analisar pacotes, remover apps ou configurar DNS.

## 8. Configurar DNS privado

Use **Configurar AdGuard DNS** e confirme para tentar configurar:

```text
dns.adguard-dns.com
```

O programa grava o modo hostname e lê os valores de volta. Isso confirma a configuração, não testa a navegação nem comprova que todos os anúncios foram bloqueados. **Voltar ao DNS automático** aplica o modo automático.

O DNS pode filtrar muitos domínios de anúncios e rastreamento, mas não remove aplicativos nem bloqueia todos os anúncios. As consultas direcionadas a esse DNS passam a usar o provedor escolhido. A configuração é global no aparelho e pode afetar outros usuários.

Se os comandos forem recusados ou a conexão falhar, configure manualmente em **Configurações → Rede e Internet/Conexões → DNS privado**, disponível no Android 9 ou superior. O nome do menu varia. Se necessário, volte a **Automático**. Em falha parcial, o programa tenta restaurar os valores anteriores e informa o resultado da tentativa.

## 9. Reduzir animações e acessibilidade

- **Ativar redução de animações:** ajusta as três escalas de animação para 0× e tenta ativar os ajustes extras de redução/remoção já expostos pelo fabricante.
- **Conferir em Acessibilidade:** abre as configurações no celular para conferir a opção.
- **Animações padrão (1×):** aplica 1× nas escalas e tenta desligar os ajustes extras expostos. Não recupera velocidades personalizadas anteriores.

No Android padrão, Remover animações de acessibilidade usa as mesmas três escalas. Em interfaces de fabricantes, a leitura de uma chave gravada não garante a posição do interruptor: confira no celular, possivelmente em **Melhorias de visibilidade** ou **Cor e movimento**. Alguns apps mantêm animações próprias.

O ajuste exige confirmação, verifica os valores gravados e informa falhas. Não habilita serviços de acessibilidade nem concede permissões a aplicativos. As escalas são globais; ajustes extras dependem do fabricante. Chaves extras ausentes não são criadas. Em falha dos ajustes extras, o programa tenta restaurá-los e informa que as escalas já podem ter sido alteradas.

## 10. Cache de Chrome, Brave e Firefox

Escolha **Cache do navegador padrão** ou um dos botões **Chrome**, **Brave** e **Firefox**. As opções explícitas não exigem que o navegador escolhido seja o padrão.

| Navegador estável | Pacote permitido |
| --- | --- |
| Chrome | `com.android.chrome` |
| Brave | `com.brave.browser` |
| Firefox | `org.mozilla.firefox` |

O programa confere a instalação para o usuário atual e pede confirmação. Se o padrão não puder ser identificado, Chrome pode ser apresentado explicitamente como alternativa, caso esteja instalado. Outros navegadores, Beta, Nightly e forks não entram nessa lista de limpeza.

A limpeza automática exige Android 13/API 33 ou superior **e** anúncio explícito de suporte a `--cache-only` no `pm help` do aparelho. Navegador, usuário e suporte são verificados novamente antes da execução.

O único comando de exclusão usado nessa função é:

```text
pm clear --user ID --cache-only PACOTE
```

**Nunca há fallback para `pm clear PACOTE`, que apagaria os dados do app.** Não é solicitada exclusão de histórico, cookies, senhas, favoritos ou downloads. O programa relata sucesso informado pelo Android, não mede o espaço liberado, e o navegador pode recriar cache.

Sem suporte confirmado, abre os detalhes do navegador no celular para você acessar **Armazenamento → Limpar cache**. Não escolha Limpar dados/armazenamento. Nesse caminho, o programa não afirma ter apagado cache. Políticas do fabricante podem impedir a limpeza mesmo quando a sintaxe é anunciada.

## 11. Limpar cache do aparelho

Essa função tem escopo limitado: **apenas miniaturas regeneráveis em duas pastas**, não todo o cache do Android.

1. Clique em **Limpar cache do aparelho** para gerar a prévia.
2. Confira os arquivos e os avisos de acesso.
3. Marque **Revisei a lista e desejo apagar somente esses arquivos de miniaturas**.
4. Clique em **Confirmar limpeza**.

Pastas permitidas, no usuário Android analisado:

```text
/storage/emulated/USUARIO/DCIM/.thumbnails
/storage/emulated/USUARIO/Pictures/.thumbnails
```

Só considera arquivos diretamente nessas pastas, com nome `.thumbdata...` ou extensão JPG/JPEG/PNG/WebP e nomes simples. Não percorre subpastas, não remove diretórios, rejeita links simbólicos e valida os caminhos novamente antes de excluir.

**Downloads/Download, WhatsApp/Business e os caches privados de bancos, Instagram e Facebook ficam fora dos alvos.** Fotos originais fora dessas pastas de miniaturas são preservadas. Se você guardou algo importante deliberadamente dentro delas, cancele e mova antes de continuar.

Cada prévia limita a seleção aos primeiros 1000 candidatos; a execução usa lotes de até 50 arquivos e verifica o usuário antes de cada lote. O resumo distingue remoções verificadas de falhas/resultados sem confirmação e não inventa uma medida de espaço liberado.

Alguns aparelhos não têm essas pastas ou não permitem acesso por ADB. Nenhum arquivo elegível/acessível encontrado não significa ausência de cache no aparelho. A galeria pode recriar as miniaturas, tornando o ganho temporário. A função não executa limpeza geral de apps, exclusão recursiva ou varredura de pastas pessoais.

## 12. Instalar Files do Google

**Instalar Files pela Play Store** abre esta página oficial no celular:

https://play.google.com/store/apps/details?id=com.google.android.apps.nbu.files

Você confirma a instalação na loja. Se já estiver instalado, ela pode oferecer Abrir ou Atualizar. O programa tenta abrir a Play Store e, em caso de falha, o mesmo link oficial no navegador.

Não baixa APKs de terceiros, não instala silenciosamente, não concede permissões e não afirma que o app foi instalado apenas por abrir a página. Internet, conta na loja e compatibilidade podem ser necessárias.

O botão **não inicia limpeza no Files**. As escolhas feitas depois dentro desse aplicativo são independentes: revise as sugestões e preserve Downloads e suas mídias. O AdwareGuard não configura exclusões nem controla a limpeza realizada por outro app.

## 13. Verificar com Google Play Protect

Clique em **Verificar com Play Protect**. O programa abre a Play Store; no celular, acesse:

```text
Perfil → Play Protect → Verificar
```

Aguarde e siga as recomendações exibidas pelo Google. O AdwareGuard não dispara uma varredura silenciosa, não acompanha o diagnóstico e não conta eventuais remoções feitas pelo Play Protect.

O aparelho precisa ter Play Store e serviços compatíveis. Abertura da loja não significa verificação iniciada ou concluída. Play Protect pode ajudar a identificar aplicativos potencialmente nocivos, mas não garante detectar todo malware.

## 14. Atualizar sem perder suas configurações locais

1. Feche o AdwareGuard.
2. Preserve o ADB/AAPT2 em `tools` e sua lista local em `data`.
3. Substitua `Cargo.toml`, `Cargo.lock` e toda a pasta `src` pela nova versão.
4. Substitua este README e o script de compilação quando forem atualizados.
5. Execute `compilar-windows.bat` e abra o novo `dist/adwareguard.exe`.

O script copia os dados do projeto para `dist`. Se você editou a lista apenas em `dist/data`, copie-a antes para `data` no projeto para não sobrescrevê-la com uma versão antiga ou vazia.

A escolha de tema vale para a sessão atual; o programa inicia no tema claro. Seleções, histórico, metadados carregados e contadores ficam em memória.

## 15. Estrutura do projeto

```text
AdwareGuard/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── compilar-windows.bat
├── src/
│   ├── main.rs
│   ├── core.rs
│   ├── icons.rs
│   ├── maintenance.rs
│   └── device_cleanup.rs
├── data/
│   └── confirmed_malware.txt
├── tools/
│   ├── platform-tools/  ← adicionar o ADB oficial
│   └── aapt2/           ← opcional
└── .github/workflows/windows.yml
```

A interface usa eframe/egui 0.31.1. O processamento ADB é executado fora da thread da interface, com outras ações ADB bloqueadas durante uma operação. Cada comando tem limite de 45 segundos; uma operação com vários comandos pode demorar mais.

## 16. Resolução de problemas

| Problema | Verifique |
| --- | --- |
| “Não foi possível iniciar ADB” / erro 2 | Caminho de `adb.exe`, extração correta e DLLs junto do EXE. |
| Celular não aparece | Cabo de dados, porta USB, driver, Depuração USB e autorização na tela. |
| Falha de compilação | Primeira mensagem de erro, Rust atualizado, Build Tools C++, Windows SDK e acesso às dependências. |
| Lista local inválida/ausente | Presença de `data/confirmed_malware.txt` e formato pacote + fonte HTTPS. |
| Nome comercial/ícone não aparece | Caminho do AAPT2 e limitações de ícones adaptativos ou APKs divididos. |
| Remoção recusada | Políticas administrativas, perfil de trabalho ou proteção do fabricante; o app não tenta contorná-las. |
| Timeout durante remoção/limpeza | Reanalise antes de repetir; parte da operação pode ter ocorrido. |
| DNS sem navegação | Volte a Automático no Android; a rede pode restringir DNS privado. |
| Redução de animações não refletida no menu | Confira Acessibilidade/Opções do desenvolvedor e as limitações do fabricante. |
| Cache do navegador sem limpeza automática | Use a opção manual no celular; não substitua por limpeza de dados. |
| Nenhuma miniatura elegível | As pastas podem estar ausentes ou inacessíveis; não há fallback para outras pastas. |
| Play Store não abre | Confira se ela está instalada/disponível para esse usuário; o programa não instala uma loja alternativa. |

## 17. Limites, privacidade e validação

- Use os alertas como apoio à revisão, não como diagnóstico definitivo.
- A lista inicial vazia significa que nenhum pacote é considerado correspondência local por padrão.
- A seleção manual não transforma um app em malware confirmado.
- O projeto não oferece serviço remoto próprio de análise. Leitura do APK para nome/ícone é local; DNS, Play Store e Play Protect envolvem os respectivos provedores.
- O histórico não é salvo automaticamente. Ao terminar, você pode desativar a Depuração USB ou revogar autorizações no Android.
- Projeto independente, sem afiliação com Google ou AdGuard.

**Validação da versão 0.7.0:** `cargo test` passou com 14 testes no Linux, incluindo teste do script de exclusão em pasta temporária, preservação de originais/Downloads e recusa de links simbólicos. `cargo check --target x86_64-pc-windows-msvc` também passou.

Isso não equivale a executar o EXE no Windows ou testar comandos num celular real. A interface, compatibilidade do fabricante, limpeza no Android e abertura da Play Store ainda precisam ser conferidas no aparelho conectado. O projeto-fonte não inclui EXE Windows compilado nem ADB/AAPT2.

## Referências oficiais

- [Instalação do Rust](https://www.rust-lang.org/tools/install)
- [Android SDK Platform-Tools](https://developer.android.com/tools/releases/platform-tools)
- [ADB e conexão com dispositivos](https://developer.android.com/tools/adb)
- [AAPT2](https://developer.android.com/tools/aapt2)
- [DNS privado no Android](https://support.google.com/android/answer/9654714)
- [DNS público AdGuard](https://adguard-dns.io/en/public-dns.html)
- [Configurações globais do Android](https://developer.android.com/reference/android/provider/Settings.Global)
- [Código oficial do comando de cache/dados do Android](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/services/core/java/com/android/server/pm/PackageManagerShellCommand.java)
- [Files do Google na Play Store](https://play.google.com/store/apps/details?id=com.google.android.apps.nbu.files)
- [Google Play Protect](https://support.google.com/googleplay/answer/2812853?hl=pt-BR)
