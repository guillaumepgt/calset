#!/bin/sh
set -e

echo "1/3 Installation de htpasswd..."
apk add --no-cache apache2-utils > /dev/null

echo "2/3 Configuration des utilisateurs et permissions..."
htpasswd -B -b -c /etc/radicale/auth/users "$$CAL_USER" "$$CAL_PASS"
chown -R 2999:2999 /etc/radicale/auth
chmod 600 /etc/radicale/auth/users

echo "3/3 Configuration de l'agenda par défaut..."
CAL_DIR="/var/lib/radicale/collections/collection-root/$$CAL_USER/agenda"
mkdir -p "$$CAL_DIR"
printf '{"tag": "VCALENDAR", "D:displayname": "Mon Agenda"}\n' > "$$CAL_DIR/.Radicale.props"
chown -R 2999:2999 /var/lib/radicale/collections

echo "Initialisation terminée avec succès."