FROM scratch
COPY ./back/back /usr/local/bin/back
COPY ./front/dist /app/front
ENV FRONTEND_DIR=/app/front
EXPOSE 8000
ENTRYPOINT ["back"]

