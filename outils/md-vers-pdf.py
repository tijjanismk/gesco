"""Compose en PDF A4 un manuel Markdown de GESCO (manuel d'utilisation, guide d'installation).

    python outils/md-vers-pdf.py docs/manuel-utilisateur-1.2.md docs/manuel-utilisateur-1.2.pdf

Le Markdown des manuels reste simple : titres #, ## et ###, paragraphes, listes à puces ou
numérotées (lignes de suite indentées), tableaux |, filets ---, **gras** et `code`. Police
Charis SIL, celle des documents de GESCO (crates/gesco-docs/polices). Demande ReportLab.
"""

import pathlib
import re
import sys

from reportlab.lib import colors
from reportlab.lib.enums import TA_LEFT
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.units import mm
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.platypus import (
    HRFlowable,
    KeepTogether,
    ListFlowable,
    ListItem,
    Paragraph,
    SimpleDocTemplate,
    Spacer,
    Table,
    TableStyle,
)

RACINE = pathlib.Path(__file__).resolve().parent.parent
POLICES = RACINE / "crates" / "gesco-docs" / "polices"
pdfmetrics.registerFont(TTFont("Charis", str(POLICES / "CharisSIL-Regular.ttf")))
pdfmetrics.registerFont(TTFont("Charis-Gras", str(POLICES / "CharisSIL-Bold.ttf")))
pdfmetrics.registerFontFamily("Charis", normal="Charis", bold="Charis-Gras")

BLEU = colors.HexColor("#1e3a8a")
TEXTE = ParagraphStyle("texte", fontName="Charis", fontSize=10, leading=13.5, alignment=TA_LEFT, spaceAfter=5)
CELLULE = ParagraphStyle("cellule", parent=TEXTE, fontSize=9, leading=11.5, spaceAfter=0)
TITRE = ParagraphStyle("titre", parent=TEXTE, fontName="Charis-Gras", fontSize=20, leading=24, textColor=BLEU, spaceAfter=10)
H2 = ParagraphStyle("h2", parent=TEXTE, fontName="Charis-Gras", fontSize=14, leading=18, textColor=BLEU, spaceBefore=12, spaceAfter=6)
H3 = ParagraphStyle("h3", parent=TEXTE, fontName="Charis-Gras", fontSize=11.5, leading=15, spaceBefore=8, spaceAfter=4)


def en_ligne(texte: str) -> str:
    """Gras et code du Markdown en balises de ReportLab, le reste échappé."""
    texte = texte.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    texte = re.sub(r"`([^`]+)`", r'<font face="Courier" size="9">\1</font>', texte)
    return re.sub(r"\*\*(.+?)\*\*", r"<b>\1</b>", texte)


def tableau(lignes: list[str]) -> Table:
    rangees = []
    for ligne in lignes:
        cellules = [c.strip() for c in ligne.strip().strip("|").split("|")]
        if all(re.fullmatch(r":?-{2,}:?", c) for c in cellules):
            continue
        rangees.append(cellules)
    donnees = [
        [Paragraph(("<b>%s</b>" if i == 0 else "%s") % en_ligne(c), CELLULE) for c in r]
        for i, r in enumerate(rangees)
    ]
    largeur = 170 * mm
    colonnes = len(rangees[0]) if rangees else 1
    parts = [0.22, 0.28, 0.50] if colonnes == 3 else [1 / colonnes] * colonnes
    t = Table(donnees, colWidths=[largeur * p for p in parts], repeatRows=1)
    t.setStyle(
        TableStyle(
            [
                ("GRID", (0, 0), (-1, -1), 0.4, colors.HexColor("#9ca3af")),
                ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#e5e7eb")),
                ("VALIGN", (0, 0), (-1, -1), "TOP"),
                ("TOPPADDING", (0, 0), (-1, -1), 3),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 3),
            ]
        )
    )
    return t


def liste(elements: list[str], numerotee: bool) -> ListFlowable:
    return ListFlowable(
        [ListItem(Paragraph(en_ligne(e), TEXTE), leftIndent=14) for e in elements],
        bulletType="1" if numerotee else "bullet",
        start="1" if numerotee else "•",
        bulletFontName="Charis",
        bulletFontSize=10,
        leftIndent=14,
    )


def composer(markdown: str) -> list:
    histoire = []
    lignes = markdown.splitlines()
    i = 0
    while i < len(lignes):
        ligne = lignes[i]
        if not ligne.strip():
            i += 1
        elif ligne.startswith("# "):
            histoire.append(Paragraph(en_ligne(ligne[2:]), TITRE))
            i += 1
        elif ligne.startswith("## "):
            histoire.append(Paragraph(en_ligne(ligne[3:]), H2))
            i += 1
        elif ligne.startswith("### "):
            histoire.append(Paragraph(en_ligne(ligne[4:]), H3))
            i += 1
        elif ligne.strip() == "---":
            histoire.append(HRFlowable(width="100%", thickness=0.6, color=colors.HexColor("#9ca3af"), spaceBefore=4, spaceAfter=8))
            i += 1
        elif ligne.startswith("|"):
            bloc = []
            while i < len(lignes) and lignes[i].startswith("|"):
                bloc.append(lignes[i])
                i += 1
            histoire.append(tableau(bloc))
            histoire.append(Spacer(1, 6))
        elif re.match(r"^(- |\d+\. )", ligne):
            numerotee = not ligne.startswith("- ")
            elements = []
            while i < len(lignes) and (re.match(r"^(- |\d+\. )", lignes[i]) or (lignes[i].startswith("  ") and lignes[i].strip())):
                if re.match(r"^(- |\d+\. )", lignes[i]):
                    elements.append(re.sub(r"^(- |\d+\. )", "", lignes[i]))
                else:
                    elements[-1] += " " + lignes[i].strip()
                i += 1
            histoire.append(liste(elements, numerotee))
            histoire.append(Spacer(1, 4))
        else:
            bloc = []
            while i < len(lignes) and lignes[i].strip() and not re.match(r"^(#|\||- |\d+\. |---)", lignes[i]):
                bloc.append(lignes[i].strip())
                i += 1
            histoire.append(Paragraph(en_ligne(" ".join(bloc)), TEXTE))
    # Un titre ne reste pas seul en bas de page.
    gardee = []
    for element in histoire:
        if gardee and isinstance(gardee[-1], Paragraph) and gardee[-1].style in (H2, H3):
            titre = gardee.pop()
            gardee.append(KeepTogether([titre, element]))
        else:
            gardee.append(element)
    return gardee


def main() -> None:
    source, cible = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    markdown = source.read_bytes().decode("utf-8")
    titre = markdown.splitlines()[0].lstrip("# ").strip()
    version = re.search(r"version (\d+\.\d+\.\d+)", markdown)
    entete = f"{titre} · version {version.group(1)}" if version else titre

    def page(canvas, document):
        canvas.saveState()
        canvas.setFont("Charis", 8)
        canvas.setFillColor(colors.HexColor("#6b7280"))
        canvas.drawString(20 * mm, A4[1] - 12 * mm, entete)
        canvas.drawRightString(A4[0] - 20 * mm, 10 * mm, f"page {document.page}")
        canvas.restoreState()

    document = SimpleDocTemplate(
        str(cible),
        pagesize=A4,
        leftMargin=20 * mm,
        rightMargin=20 * mm,
        topMargin=18 * mm,
        bottomMargin=16 * mm,
        title=titre,
        author="GESCO",
    )
    document.build(composer(markdown), onFirstPage=page, onLaterPages=page)


if __name__ == "__main__":
    main()
