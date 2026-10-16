FROM debian:bookworm-slim

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        inetutils-telnetd inetutils-inetd login passwd procps ncurses-bin vim-tiny \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -s /bin/bash termvexa \
    && passwd -l root \
    && printf 'telnet stream tcp nowait root /usr/sbin/telnetd telnetd\n' > /etc/inetd.conf \
    && printf 'Termvexa local Telnet test server\n' > /etc/issue.net

COPY profile /home/termvexa/.profile
RUN chown termvexa:termvexa /home/termvexa/.profile \
    && ln -s /usr/bin/vim.tiny /usr/local/bin/vim

ENV LANG=C.UTF-8 TERM=xterm-256color
EXPOSE 23

CMD ["/usr/sbin/inetutils-inetd", "-d", "/etc/inetd.conf"]
