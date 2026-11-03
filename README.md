# RUN

```bash
docker compose -f radicale/init.yml run --rm init
docker compose up -d
```

# TEST

```bash
curl -i -u mon_utilisateur:mon_mot_de_passe_robuste http://127.0.0.1:5232/
```