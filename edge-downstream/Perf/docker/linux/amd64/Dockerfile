ARG base_tag=2.1.10-alpine3.7
FROM mcr.microsoft.com/dotnet/core/runtime:${base_tag}

ARG EXE_DIR=./bin/Debug/netcoreapp2.1/publish

ENV MODULE_NAME "PerfMessageGenerator.dll"

WORKDIR /app

COPY $EXE_DIR/ ./

ADD ./generated/azure-iot-test-only.root.ca.cert.pem.crt /usr/local/share/ca-certificates/azure-iot-test-only.root.ca.cert.pem.crt
RUN chmod 644 /usr/local/share/ca-certificates/azure-iot-test-only.root.ca.cert.pem.crt && update-ca-certificates

# Add an unprivileged user account for running the module
RUN adduser -Ds /bin/sh moduleuser 
USER moduleuser

CMD echo "$(date --utc +"[%Y-%m-%d %H:%M:%S %:z]"): Starting Module" && \
    exec /usr/bin/dotnet PerfMessageGenerator.dll
