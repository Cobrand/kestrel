# Kestrel

Transport your data over UDP with goodies. It was designed for real-time multiplayer games,
but can also be used for VoIP instead.

Kestrel has these basic goals:

* Cross platform: Linux, macOS, Windows (MSVC)
* Guarantee of non-corrupted packages with crc32.
* Packet fragmentation for message having a length higher than MTU.
* Packet re-ordering.
* Optional protocol ID to avoid having 2 versions clash.
* Congestion tracking & prevention.
* Optional Packet re-sending, with forgettable packets, timeout-able "key" and true "key" packets
* Priority handling: auto-dropping of packets when the receiver is in congested mode

It is close but different from `cobalt-rs`, please check it out as well.

