// Material Symbols (Outlined, weight 500, Apache-2.0), embedded at build time, plus the
// Windows-style caption glyphs.
import play from "@material-symbols/svg-500/outlined/play_arrow-fill.svg?raw";
import pause from "@material-symbols/svg-500/outlined/pause-fill.svg?raw";
import skipNext from "@material-symbols/svg-500/outlined/skip_next-fill.svg?raw";
import skipPrev from "@material-symbols/svg-500/outlined/skip_previous-fill.svg?raw";
import volumeUp from "@material-symbols/svg-500/outlined/volume_up.svg?raw";
import volumeDown from "@material-symbols/svg-500/outlined/volume_down.svg?raw";
import volumeOff from "@material-symbols/svg-500/outlined/volume_off.svg?raw";
import fullscreen from "@material-symbols/svg-500/outlined/fullscreen.svg?raw";
import fullscreenExit from "@material-symbols/svg-500/outlined/fullscreen_exit.svg?raw";
import folderOpen from "@material-symbols/svg-500/outlined/folder_open.svg?raw";
import settings from "@material-symbols/svg-500/outlined/settings.svg?raw";
import musicNote from "@material-symbols/svg-500/outlined/music_note.svg?raw";
import subtitles from "@material-symbols/svg-500/outlined/subtitles.svg?raw";
import audio from "@material-symbols/svg-500/outlined/graphic_eq.svg?raw";
import speed from "@material-symbols/svg-500/outlined/speed.svg?raw";
import repeat from "@material-symbols/svg-500/outlined/repeat.svg?raw";
import keep from "@material-symbols/svg-500/outlined/keep.svg?raw";
import camera from "@material-symbols/svg-500/outlined/photo_camera.svg?raw";
import chevronRight from "@material-symbols/svg-500/outlined/chevron_right.svg?raw";
import check from "@material-symbols/svg-500/outlined/check.svg?raw";
import logout from "@material-symbols/svg-500/outlined/logout.svg?raw";
import movie from "@material-symbols/svg-500/outlined/movie.svg?raw";
import replay from "@material-symbols/svg-500/outlined/replay.svg?raw";
import info from "@material-symbols/svg-500/outlined/info.svg?raw";

const caption = (d: string) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10" class="cap" fill="none" stroke="currentColor" stroke-width="1">${d}</svg>`;

export const ICONS: Record<string, string> = {
  play,
  pause,
  skip_next: skipNext,
  skip_prev: skipPrev,
  volume_up: volumeUp,
  volume_down: volumeDown,
  volume_off: volumeOff,
  fullscreen,
  fullscreen_exit: fullscreenExit,
  folder_open: folderOpen,
  settings,
  music_note: musicNote,
  subtitles,
  audio,
  speed,
  repeat,
  keep,
  camera,
  chevron_right: chevronRight,
  check,
  logout,
  movie,
  replay,
  info,
  cap_min: caption('<path d="M0 5.5h10"/>'),
  cap_max: caption('<rect x="0.5" y="0.5" width="9" height="9"/>'),
  cap_restore: caption('<rect x="0.5" y="2.5" width="7" height="7"/><path d="M2.5 2.5V0.5h7v7h-2"/>'),
  cap_close: caption('<path d="M0.5 0.5l9 9M9.5 0.5l-9 9"/>'),
};
