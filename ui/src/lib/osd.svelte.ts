// On-screen messages over the video (volume, speed...): one at a time, fading out.
class Osd {
  text = $state("");
  visible = $state(false);
  private timer = 0;

  show(text: string, ms = 1200) {
    this.text = text;
    this.visible = true;
    clearTimeout(this.timer);
    this.timer = window.setTimeout(() => (this.visible = false), ms);
  }
}

export const osd = new Osd();
