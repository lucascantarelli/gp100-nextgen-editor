/**
 * Ritmos da bateria (drum) da GP-100 — GERADO do firmware V2.1
 * (tabela de estilos entre as divisões de nota e os compassos).
 * NÃO editar mão. Regenerar: uv run python analysis/dump_drum_kit.py
 *
 * Os RÓTULOS de gênero são inferidos (o firmware ordena os estilos por
 * grupo sem gravar os nomes); a LISTA e a ORDEM dos estilos são reais.
 */

export interface DrumGenre {
  genre: string;
  styles: string[];
}

export const DRUM_GENRES: DrumGenre[] = [
  { genre: "Electronic", styles: ["Metro", "Electro1", "Electro2", "Techno", "TripHop", "H-Hop1", "H-Hop2", "H-Hop3", "H-Hop4", "D&B", "Break", "E-Pop"] },
  { genre: "Rock", styles: ["Surfin", "Hard 1", "Rock 1", "P Punk 1", "Punk 1", "Nu 1", "P Rock1", "Metal1", "Hard 2", "Rock 2", "P Punk 2", "Punk 2", "Nu 2", "P Rock2", "Metal2", "Hard 3", "Rock 3", "Punk 3", "P Rock3", "Punk 4", "SF3/4", "SF4/4", "Rock5/4", "Punk 5", "EMO", "R'n'R", "Classic", "Ballad", "Shuffle", "Core", "NWave", "Garag", "Prog"] },
  { genre: "Pop", styles: ["Funk 1", "Funk 2", "Funk 3", "Funk 4", "Pop 1", "Pop 2", "Pop 3", "Pub", "Blues 1", "Blues 2", "Blues 3", "Blues 4", "Folk", "B-grass"] },
  { genre: "World", styles: ["March 1", "Latin 1", "RAG1", "Bossa1", "NuAge1", "March 2", "Latin 2", "RAG2", "Bossa2", "NuAge2", "Latin 3", "Samba", "Ska", "Polka", "Mazuke", "Beguine", "Musette", "Tango", "Army", "Waltz"] },
  { genre: "Jazz", styles: ["Jazz 1", "Funk1", "Jazz 2", "Funk2", "Jazz 3", "Funk3", "Jazz 4", "Fusion"] },
];

export const DRUM_TOTAL_STYLES = 87;

/** Compassos do drum (firmware: 2/4…9/8). */
export const DRUM_BEATS = ["2/4", "3/4", "4/4", "6/4", "7/4", "6/8", "7/8", "9/8"] as const;
