// Registro MEDIDO (15/10/2026): este arquivo existe para provar a FORMA de agente
// de projeto que o cliente do Freebuff/Codebuff documenta — e que ele NÃO carrega.
//
// Três sondas, três recusas (§1 de .agents/agents/README.md):
//   .agents/poc-pesquisador.ts        -> Agent "poc-pesquisador" is not available to spawn
//   .agents/agents/poc-pesquisador.ts -> Agent "poc-pesquisador-dir" is not available to spawn
//   .agents/agents/poc-pesquisador.md -> Agent "poc-pesquisador-md" is not available to spawn
// A única spawnável é a que o cliente injeta (`researcher-web`).
//
// Não é código executado em lugar nenhum: é o artefato da medição. Se um dia o
// cliente passar a carregar agente de projeto, este arquivo é o ponto de partida
// do adaptador fino descrito na §5 do README da equipe.
export default {
  id: "poc-pesquisador",
  displayName: "POC — Pesquisador",
  // O `model` do agente é um ID OPACO no estado deste cliente
  // (`"model": "m-096e75164d"` em ~/.config/freebuff-desktop/state.json): não há
  // slug público para espelhar, então a POC herda o modelo da sessão.
  toolNames: ["web_search", "read_url"],
  spawnerPrompt: "Spawn para responder uma pergunta externa com fonte citada.",
  systemPrompt:
    "Responda em PT-BR, bullets curtos, e cite a URL de cada fato. " +
    "Se nada oficial existir, escreva 'não achei documentação oficial'.",
  instructionsPrompt: "Devolva fato → URL, sem opinião e sem preço inventado.",
};
