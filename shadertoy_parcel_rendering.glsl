#define MIN_FLOAT -1e38
#define MAX_FLOAT 1e38

#define MAX_STEPS 2000
#define ISOSURFACE 0.5
#define STEP_SIZE 0.005
#define MAX_CANDIDATES 10
#define PARCEL_BOUND_SIGMAS 3.0

//------------------------------------------------------------

struct ParcelShape {
    vec3 forward;
    vec3 normal;

    // note these are measured in standard deviations, not spatial distances
    float length;
    float width;
    float height;
};

struct Parcel {
    vec3 mean;
    mat3 covariance;
    mat3 inverseCovariance;
    float peakDensity;
    vec3 color;
};

struct ParcelIntersection {
    bool isSurface;
    float density;
    vec3 point;
    vec3 normal;
};

struct ParcelBundle {
    Parcel[MAX_CANDIDATES] parcels;
    int size;
};

struct Ray {
    vec3 origin;
    vec3 direction; // Normalized
};

//------------------------------------------------------------

vec2 distanceToBounds(
    Ray ray,
    Parcel parcel,
    float sigmas
) {
    mat3 invCov = parcel.inverseCovariance;

    vec3 oc = ray.origin - parcel.mean;
    vec3 d = ray.direction;

    float A = dot(d, invCov * d);
    float B = 2.0 * dot(d, invCov * oc);
    float C = dot(oc, invCov * oc) - sigmas * sigmas;

    float discriminant = B * B - 4.0 * A * C;

    if (discriminant < 0.0)
        return vec2(MIN_FLOAT, MAX_FLOAT); // no intersection

    float s = sqrt(discriminant);

    float t0 = (-B - s) / (2.0 * A);
    float t1 = (-B + s) / (2.0 * A);

    return vec2(min(t0, t1), max(t0, t1));
}

float parcelDensity(vec3 p, Parcel parcel) {
    vec3 d = p - parcel.mean;
    mat3 invCov = parcel.inverseCovariance;
    float mahalanobisDistanceSquared = dot(d, invCov * d);
    float falloff = exp(-0.5 * mahalanobisDistanceSquared);
    return parcel.peakDensity * falloff;
}

vec2 normalizeScreenCoordinates(vec2 fragCoord) {
    return (fragCoord - 0.5 * iResolution.xy) / iResolution.y;
}

// Gram-Schmidt orthogonalization
// returns v2 stripped of any portion pointing toward v1
vec3 orthogonalize(vec3 v1, vec3 v2) {
    return normalize(v2 - dot(v2, v1) * v1);
}

mat3 covarianceFromShape(ParcelShape shape) {
    vec3 forward = normalize(shape.forward);
    vec3 normal = orthogonalize(forward, shape.normal);
    vec3 right = cross(forward, normal);
    
    float l2 = shape.length * shape.length;
    float w2 = shape.width * shape.width;
    float t2 = shape.height * shape.height;
    
    return
          l2 * outerProduct(forward, forward)
        + w2 * outerProduct(right, right)
        + t2 * outerProduct(normal, normal);
}

vec3 densityGradient(vec3 p, ParcelBundle bundle) {
    vec3 g = vec3(0.0);

    for (int i = 0; i < bundle.size; i++) {
        Parcel parcel = bundle.parcels[i];
        vec3 d = p - parcel.mean;
        float rho = parcelDensity(p, parcel);

        g += -rho * (parcel.inverseCovariance * d);
    }

    return g;
}

vec3 normalAtPoint(vec3 p, ParcelBundle bundle) {
    return normalize(-densityGradient(p, bundle));
}

vec2 calculateParcelDistanceBounds(Ray ray, ParcelBundle bundle) {
    vec2 bounds = vec2(MIN_FLOAT, MAX_FLOAT);
    for(int i = 0; i < bundle.size; i++) {
        Parcel parcel = bundle.parcels[i];
        vec2 pb = distanceToBounds(ray, parcel, PARCEL_BOUND_SIGMAS);
        if(bounds.x == MIN_FLOAT)
            bounds.x = pb.x;
        else if(pb.x != MIN_FLOAT)
            bounds.x = min(bounds.x, pb.x);
        if(bounds.y == MAX_FLOAT)
            bounds.y = pb.y;
        else if(pb.y != MAX_FLOAT)
            bounds.y = max(bounds.y, pb.y);
    }
    return bounds;
}

ParcelIntersection traceParcels(Ray ray, ParcelBundle bundle) {
    ParcelIntersection intersection;
    intersection.isSurface = false;
    intersection.density = 0.0;
    
    vec2 range = calculateParcelDistanceBounds(ray, bundle);
    if(range.x == MIN_FLOAT || range.y == MAX_FLOAT)
        return intersection;   

    float t = range.x;
    float maxDensity = 0.0;

    for (int i = 0; i < MAX_STEPS; i++) {
        vec3 p = ray.origin + t * ray.direction;
        float d = 0.0;
        for(int j = 0; j < bundle.size; j++)
            d += parcelDensity(p, bundle.parcels[j]);
        maxDensity = max(maxDensity, d);

        if (d > ISOSURFACE) {
            intersection.isSurface = true;
            intersection.density = maxDensity;
            intersection.point = p;
            intersection.normal = normalAtPoint(p, bundle);
            return intersection;
        }

        t += STEP_SIZE;
        if(t > range.y)
            break;
    }
    
    intersection.density = maxDensity;
    return intersection;
}

float getLight(vec3 p, vec3 normal) {
    vec3 lightPos = vec3(0, 6, 0);
    vec3 lightVector = normalize(lightPos - p);
    float diffuse = clamp(dot(normal, lightVector), 0.0, 1.0);
    return diffuse;
}

Parcel[3] getParcels() {    
    float c = cos(0.8 * iTime);
    float s = sin(0.8 * iTime);
 
    ParcelShape shape1;
    shape1.forward = vec3(0,0,-1);
    shape1.normal = vec3(0,1,0);
    shape1.length = 0.3;
    shape1.width = 0.3;
    shape1.height = 0.02;
    Parcel parcel1;
    parcel1.mean = vec3(-0.6 * c,1,-3);
    parcel1.covariance = covarianceFromShape(shape1);
    parcel1.inverseCovariance = inverse(parcel1.covariance);
    parcel1.peakDensity = 0.8;
    parcel1.color = vec3(0.25, 0.75, 0.35);

    ParcelShape shape2;
    shape2.forward = vec3(0,0,-1);
    shape2.normal = vec3(0,1,0);
    shape2.length = 0.3;
    shape2.width = 0.3;
    shape2.height = 0.02;
    Parcel parcel2;
    parcel2.mean = vec3(0.6 * c,1,-3);
    parcel2.covariance = covarianceFromShape(shape2);
    parcel2.inverseCovariance = inverse(parcel2.covariance);
    parcel2.peakDensity = 0.8;
    parcel2.color = vec3(0.25, 0.75, 0.35);
    
    ParcelShape shape3;
    shape3.forward = vec3(0,0,-1);
    shape3.normal = vec3(0,1,0);
    shape3.length = 0.3;
    shape3.width = 0.02;
    shape3.height = 0.05;
    Parcel parcel3;
    parcel3.mean = vec3(0,1,-3);
    parcel3.covariance = covarianceFromShape(shape3);
    parcel3.inverseCovariance = inverse(parcel3.covariance);
    parcel3.peakDensity = 0.8;
    parcel3.color = vec3(0.25, 0.75, 0.35);

    return Parcel[3](parcel1, parcel2, parcel3);
}

void mainImage(out vec4 fragColor, in vec2 fragCoord)
{
    vec2 uv = normalizeScreenCoordinates(fragCoord);
   
    ParcelBundle bundle;
    bundle.size = 3;
    Parcel[3] threeParcels = getParcels();
    for(int i = 0; i < 3; i++)
        bundle.parcels[i] = threeParcels[i];
    
    vec3 cameraPosition = vec3(.5,1.5,0);
    vec3 cameraDirection = normalize(vec3(uv.x, uv.y, -1));
    Ray ray = Ray(cameraPosition, cameraDirection);
    ParcelIntersection intersection = traceParcels(ray, bundle);
    
    float color;
    if(intersection.isSurface) {
        float light = getLight(intersection.point, intersection.normal);
        color = max(light, intersection.density);
    }
    else {
        color = intersection.density;
    }
    
    fragColor = vec4(vec3(color * vec3(0.25, 0.75, 0.35)), 1.0);
}
