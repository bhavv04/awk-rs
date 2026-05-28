BEGIN { print "start" }
{ print $1 }
END { print "end" }