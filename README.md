<h1 align="center">WordBook.it</h1>

<p align="center">
  <b>Programma open source di videoscrittura in stile Word, scritto in Rust.</b>
</p>

WordBook.it è una versione personalizzata di [WordCraft](https://github.com/storytold/wordcraft), la
reimplementazione open source di un programma di videoscrittura scritta in puro Rust dall'ArtCraft Team.
Funziona in modo nativo su Windows, macOS e Linux, con la barra multifunzione, gli stili, le tabelle, le
revisioni, i riferimenti e la stampa unione. Usa il formato **.docx** come formato nativo, quindi i
file si scambiano normalmente con Word.

## Cosa cambia rispetto a WordCraft

- **Nome, icona e titolo** propri: la barra del titolo riporta «WordBook.it di Lorenzo D'Apolito».
- **Nome utente di Windows** al posto di «WordCraft User»: l'avatar in alto a sinistra mostra le tue iniziali
  e i commenti e le revisioni sono firmati con il tuo nome (si cambia da File → Opzioni → Generale).
- **Programma `WordBook.it.exe`**, con le informazioni di versione di Windows.
- **Modalità portatile**: una cartella `wordbook-data` accanto al programma contiene tutto il profilo
  (impostazioni, dizionari personali, copie di recupero).
- **Dizionari condivisi**: il correttore ortografico legge i dizionari Hunspell dalla cartella
  `dictionaries` accanto al programma, la stessa del pacchetto [DocPlane.it](https://github.com/lorenzodapolito-oss/DocPlane).
- **Nessuna finestra «Recupera documenti» all'avvio**: le copie di recupero restano disponibili da
  File → Recupera, ma non si aprono più da sole.

Tutto il resto, cioè motore di impaginazione, stili, tabelle, revisioni, riferimenti, esportazione in
PDF e altro, è il lavoro di WordCraft, che è un progetto in sviluppo: alcune funzioni sono ancora
incomplete e parte dell'interfaccia può apparire in inglese.

## Scarica per Windows

**[Scarica DocPlane.it (zip)](https://github.com/lorenzodapolito-oss/DocPlane/releases/latest/download/DocPlane.it.zip)**
· [tutte le versioni](https://github.com/lorenzodapolito-oss/DocPlane/releases)

WordBook.it è nel pacchetto **DocPlane.it**, insieme a SheetCalc.it, Flickdeck.it e ai dizionari.
Decomprimi lo zip in una cartella e apri `DocPlane.it.exe`, oppure direttamente `WordBook.it.exe`.
Al primo avvio Windows può mostrare «Windows ha protetto il PC» (i programmi non sono firmati):
scegli *Ulteriori informazioni*, poi *Esegui comunque*.

## Come compilare su Windows

Servono [Rust](https://rustup.rs) (toolchain `x86_64-pc-windows-gnu`) e un compilatore C per Windows,
ad esempio [WinLibs](https://winlibs.com) (UCRT, 64 bit) in una cartella **senza spazi nel percorso**,
oppure MSYS2.

```
git clone https://github.com/lorenzodapolito-oss/WordBook.it
cd WordBook.it
set PATH=<cartella di WinLibs>\mingw64\bin;%PATH%
packaging\windows\WordBook.bat
```

Il file finito è `dist\WordBook.it\WordBook.it.exe`. La prima compilazione richiede parecchi minuti.
Per provare il programma senza creare il pacchetto:

```
cargo run --release -p wordcraft -- --sample
```

## Modalità portatile

Per tenere impostazioni e dati accanto al programma, crea una cartella chiamata `wordbook-data` nella
stessa cartella di `WordBook.it.exe`. Cancellala per tornare al comportamento normale, con le
impostazioni nel profilo di Windows. I dizionari possono stare in `wordbook-data\dictionaries` oppure in
`dictionaries` accanto al programma.

## Dizionari

I dizionari non fanno parte del repository: sono quelli del repository
[LibreOffice/dictionaries](https://github.com/LibreOffice/dictionaries), ognuno con la propria licenza
(l'italiano è GPL 3). Si scaricano da DocPlane.it, con il pulsante «Scarica i dizionari», oppure si
copiano a mano i file Hunspell della lingua (per esempio `it_IT.aff` e `it_IT.dic`) nella cartella
`dictionaries`.

## Per chi vuole approfondire

La documentazione tecnica del progetto originale (architettura, formati, canale di controllo per gli
agenti, MCP, versione web) è nella cartella [`docs/`](docs/) e nel
[repository di WordCraft](https://github.com/storytold/wordcraft). Le regole del progetto, in inglese,
sono in [AGENTS.md](AGENTS.md).

## Licenza e crediti

WordBook.it è distribuito con la stessa licenza di WordCraft: [MIT](LICENSE-MIT) oppure
[Apache-2.0](LICENSE-APACHE), a scelta. Copyright (c) 2026 ArtCraft Team e contributori di WordCraft;
adattamento di Lorenzo D'Apolito. Gli avvisi richiesti sono in [NOTICE](NOTICE) e le risorse (immagini,
icone) sono elencate in [ATTRIBUTION.md](ATTRIBUTION.md). Le icone sono opera di Lorenzo D'Apolito.

Il nome ArtCraft e i suoi loghi sono marchi dell'ArtCraft Team e non sono coperti da questa licenza.
In questa versione modificata i loghi sono stati rimossi, come richiesto dalle
[condizioni d'uso del marchio](docs/brand/LICENSE-brand.txt).
