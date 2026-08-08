const videoElem = document.getElementById('video');
const statusElem = document.getElementById('status');

async function startPlayer() {
  if (typeof mpegts !== 'undefined' && mpegts.isSupported()) {

    const player = mpegts.createPlayer({
      type: 'flv',
      isLive: true,
      url: 'https://sapa-tv.ru/live/stream1.flv'
    }, {
      enableStashBuffer: false,
      liveBufferLatencyChasing: true
    });

    player.attachMediaElement(videoElem);
    player.load();
    player.play().catch(e => console.log("Autoplay blocked:", e));
  } else {
    console.error("mpegts.js не загружен или не поддерживается браузером");
  }
}

document.addEventListener('DOMContentLoaded', () => {
  startPlayer();

  const vkBtn = document.getElementById('vk-chat-btn');
  vkBtn.addEventListener('click', () => {
    window.open('https://live.vkvideo.ru/sapushka_/stream/default/only-chat', '_blank');
  });
});