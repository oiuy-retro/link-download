# Known issues

Open copies live on the [Issues](https://github.com/oiuy-retro/link-download/issues) tab.

## Custom mouse cursor

[#1](https://github.com/oiuy-retro/link-download/issues/1). Enhancement; no branch yet.

## Progress bar last block

[#33](https://github.com/oiuy-retro/link-download/issues/33). The bar at the bottom of the Download page leaves its last navy block empty when the run has finished. This branch fills that block at 100%.

## Retry failed downloads

[#34](https://github.com/oiuy-retro/link-download/issues/34). When a download run finishes with errors, Start over returns to Options and drops the fetched names. This branch shows Retry instead and downloads only the links that failed.

## Time left seconds and items per second

[#35](https://github.com/oiuy-retro/link-download/issues/35). Time left includes seconds, and the item rate is per second, so both numbers jump. This branch shows Time left in days, hours, and minutes, and counts items per minute.

## Progress bar right edge

[#36](https://github.com/oiuy-retro/link-download/issues/36). At 100% the last navy piece is a short stub after a gap, so it does not meet the right edge. This branch joins that stub to the block before it. While the bar is still filling, it may stop partway through a block.

## Items per minute on a short run

[#37](https://github.com/oiuy-retro/link-download/issues/37). One finished item in about ten seconds shows as 5.77 items per minute, because that one item is stretched out to a full minute. This branch does not stretch a run that is shorter than a minute, so one item shows as 1.

## Download error cut off

[#38](https://github.com/oiuy-retro/link-download/issues/38). An error on a download row is drawn only in the Download and Convert columns, and widening the window grows the File column, so the rest of the message stays cut off. This branch stops a short file name at the text and gives the rest of the row to the error.

## Progress bar last block is too thick

[#39](https://github.com/oiuy-retro/link-download/issues/39). At 100% the last navy block stretches to the right edge, so it is about twice as wide as the others and changes width with the window. This branch keeps every block 8 pixels wide. Spare pixels go into the gaps, and the last block still ends on the inner right edge.
