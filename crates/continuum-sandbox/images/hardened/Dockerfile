FROM continuum-sandbox-runtime:latest AS hardened

RUN apt-get update && apt-get install -y --no-install-recommends \
    apparmor apparmor-profiles apparmor-utils \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /etc/apparmor.d && \
    echo "profile sandbox /usr/local/bin/sandbox-entrypoint.sh flags=(attach_disconnected) {" > /etc/apparmor.d/sandbox && \
    echo "  /usr/bin/** ix," >> /etc/apparmor.d/sandbox && \
    echo "  /usr/local/** ix," >> /etc/apparmor.d/sandbox && \
    echo "  /workspace/** rwix," >> /etc/apparmor.d/sandbox && \
    echo "  /cache/** rwk," >> /etc/apparmor.d/sandbox && \
    echo "  deny /etc/shadow r," >> /etc/apparmor.d/sandbox && \
    echo "  deny /etc/sudoers r," >> /etc/apparmor.d/sandbox && \
    echo "  deny /root/** r," >> /etc/apparmor.d/sandbox && \
    echo "  deny /home/** r," >> /etc/apparmor.d/sandbox && \
    echo "}" >> /etc/apparmor.d/sandbox

RUN useradd -m -u 1000 -s /bin/bash sandbox

USER sandbox
WORKDIR /workspace
COPY --from=continuum-sandbox-runtime:latest /usr/local/bin/sandbox-entrypoint.sh /usr/local/bin/sandbox-entrypoint.sh
ENTRYPOINT ["sandbox-entrypoint.sh"]
